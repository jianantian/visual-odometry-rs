use itertools::izip;
use nalgebra::{DMatrix, Isometry3, Matrix6, Point2, UnitQuaternion, Vector6};
use std::f32;

use crate::camera::Intrinsics;
use crate::inverse_depth::{self, InverseDepth};
use crate::optimization_bis::{self as optim, Optimizer};
use crate::{candidates, helper, multires, se3};

type Levels<T> = Vec<T>;
type Vec6 = Vector6<f32>;
type Mat6 = Matrix6<f32>;
type Iso3 = Isometry3<f32>;

pub struct Tracker {
    config: Config,
    state: State,
}

pub struct Config {
    pub nb_levels: usize,
    pub candidates_diff_threshold: u16,
    pub depth_scale: f32,
    pub intrinsics: Intrinsics,
}

pub struct State {
    keyframe_multires_data: MultiresData,
    keyframe_depth_timestamp: f64,
    keyframe_img_timestamp: f64,
    keyframe_pose: Iso3,
    current_frame_depth_timestamp: f64,
    current_frame_img_timestamp: f64,
    current_frame_pose: Iso3,
}

struct MultiresData {
    intrinsics_multires: Levels<Intrinsics>,
    img_multires: Levels<DMatrix<u8>>,
    gradients_multires: Levels<(DMatrix<i16>, DMatrix<i16>)>,
    gradients_squared_norm_multires: Levels<DMatrix<u16>>,
    candidates_points: DMatrix<bool>,
    usable_candidates_multires: Levels<(Vec<(usize, usize)>, Vec<f32>)>,
    jacobians_multires: Levels<Vec<Vec6>>,
    hessians_multires: Levels<Vec<Mat6>>,
}

impl Config {
    pub fn init(
        self,
        keyframe_depth_timestamp: f64,
        depth_map: DMatrix<u16>,
        keyframe_img_timestamp: f64,
        img: DMatrix<u8>,
    ) -> Tracker {
        // Precompute multi-resolution first frame data.
        let intrinsics_multires = self.intrinsics.clone().multi_res(self.nb_levels);
        let img_multires = multires::mean_pyramid(self.nb_levels, img);
        let keyframe_multires_data =
            precompute_multires_data(&self, &depth_map, intrinsics_multires, img_multires);

        // Regroup everything under the returned Tracker.
        Tracker {
            state: State {
                keyframe_multires_data,
                keyframe_depth_timestamp,
                keyframe_img_timestamp,
                keyframe_pose: Iso3::identity(),
                current_frame_depth_timestamp: keyframe_depth_timestamp,
                current_frame_img_timestamp: keyframe_img_timestamp,
                current_frame_pose: Iso3::identity(),
            },
            config: self,
        }
    }
} // impl Config

fn precompute_multires_data(
    config: &Config,
    depth_map: &DMatrix<u16>,
    intrinsics_multires: Levels<Intrinsics>,
    img_multires: Levels<DMatrix<u8>>,
) -> MultiresData {
    // Precompute multi-resolution of keyframe gradients.
    let mut gradients_multires = multires::gradients_xy(&img_multires);
    gradients_multires.insert(0, im_gradient(&img_multires[0]));
    let gradients_squared_norm_multires: Vec<_> = gradients_multires
        .iter()
        .map(|(gx, gy)| grad_squared_norm(gx, gy))
        .collect();

    // Precompute mask of candidate points for tracking.
    let candidates_points = candidates::select(
        config.candidates_diff_threshold,
        &gradients_squared_norm_multires,
    )
    .pop()
    .unwrap();

    // Only keep the "usable" points, i.e. those with a known depth information.
    let from_depth = |z| inverse_depth::from_depth(config.depth_scale, z);
    let idepth_candidates = helper::zip_mask_map(
        &depth_map,
        &candidates_points,
        InverseDepth::Unknown,
        from_depth,
    );
    let fuse = |a, b, c, d| inverse_depth::fuse(a, b, c, d, inverse_depth::strategy_dso_mean);
    let idepth_multires = multires::limited_sequence(
        config.nb_levels,
        idepth_candidates,
        |m| m,
        |m| multires::halve(&m, fuse),
    );
    let usable_candidates_multires: Levels<_> = idepth_multires.iter().map(extract_z).collect();

    // Precompute the Jacobians.
    let jacobians_multires: Levels<Vec<Vec6>> = izip!(
        &intrinsics_multires,
        &usable_candidates_multires,
        &gradients_multires,
    )
    .map(|(intrinsics, (coord, _z), (gx, gy))| warp_jacobians(intrinsics, coord, _z, gx, gy))
    .collect();

    // Precompute the Hessians.
    let hessians_multires: Levels<_> = jacobians_multires.iter().map(hessians_vec).collect();

    // Regroup everything under a MultiresData.
    MultiresData {
        intrinsics_multires,
        img_multires,
        gradients_multires,
        gradients_squared_norm_multires,
        candidates_points,
        usable_candidates_multires,
        jacobians_multires,
        hessians_multires,
    }
}

impl Tracker {
    pub fn track(
        &mut self,
        depth_time: f64,
        depth_map: DMatrix<u16>,
        img_time: f64,
        img: DMatrix<u8>,
    ) {
        let mut lm_model = self.state.current_frame_pose.inverse() * self.state.keyframe_pose;
        let img_multires = multires::mean_pyramid(self.config.nb_levels, img);
        let keyframe_data = &self.state.keyframe_multires_data;
        let mut optimization_went_well = true;
        for lvl in (0..self.config.nb_levels).rev() {
            let obs = Obs {
                intrinsics: &keyframe_data.intrinsics_multires[lvl],
                template: &keyframe_data.img_multires[lvl],
                image: &img_multires[lvl],
                coordinates: &keyframe_data.usable_candidates_multires[lvl].0,
                _z_candidates: &keyframe_data.usable_candidates_multires[lvl].1,
                jacobians: &keyframe_data.jacobians_multires[lvl],
                hessians: &keyframe_data.hessians_multires[lvl],
            };
            let data = LMOptimizer::init(&obs, lm_model).unwrap();
            let lm_state = LMState { lm_coef: 0.1, data };
            match LMOptimizer::iterative(&obs, lm_state) {
                Some((lm_state, _)) => {
                    lm_model = lm_state.data.model;
                }
                None => {
                    eprintln!("Iterations did not converge!");
                    optimization_went_well = false;
                    break;
                }
            }
        }

        // Update current frame info in tracker.
        self.state.current_frame_depth_timestamp = depth_time;
        self.state.current_frame_img_timestamp = img_time;
        if optimization_went_well {
            self.state.current_frame_pose = self.state.keyframe_pose * lm_model.inverse();
        }

        // Check if we need to change the keyframe.
        let (coordinates, _z_candidates) = keyframe_data.usable_candidates_multires.last().unwrap();
        let intrinsics = keyframe_data.intrinsics_multires.last().unwrap();
        let optical_flow_sum: f32 = _z_candidates
            .iter()
            .zip(coordinates.iter())
            .map(|(&_z, &(x, y))| {
                let (u, v) = warp(&lm_model, x as f32, y as f32, _z, intrinsics);
                (x as f32 - u).abs() + (y as f32 - v).abs()
            })
            .sum();
        let optical_flow = optical_flow_sum / _z_candidates.len() as f32;
        eprintln!("Optical_flow: {}", optical_flow);

        let change_keyframe = optical_flow >= 1.0;

        // In case of keyframe change, update all keyframe info with current frame.
        if change_keyframe {
            let delta_time = depth_time - self.state.keyframe_depth_timestamp;
            eprintln!("Changing keyframe after: {} seconds", delta_time);
            self.state.keyframe_multires_data = precompute_multires_data(
                &self.config,
                &depth_map,
                keyframe_data.intrinsics_multires.clone(),
                img_multires,
            );
            self.state.keyframe_depth_timestamp = depth_time;
            self.state.keyframe_img_timestamp = img_time;
            self.state.keyframe_pose = self.state.current_frame_pose;
        }
    }

    pub fn current_frame(&self) -> (f64, Iso3) {
        (
            self.state.current_frame_depth_timestamp,
            self.state.current_frame_pose,
        )
    }
} // impl Tracker

// OPTIMIZATION ################################################################

struct LMOptimizer;
struct LMState {
    lm_coef: f32,
    data: LMData,
}
struct LMData {
    hessian: Mat6,
    gradient: Vec6,
    energy: f32,
    model: Iso3,
}
type LMPartialState = Result<LMData, f32>;
type PreEval = (Vec<usize>, Vec<f32>, f32);
struct Obs<'a> {
    intrinsics: &'a Intrinsics,
    template: &'a DMatrix<u8>,
    image: &'a DMatrix<u8>,
    coordinates: &'a Vec<(usize, usize)>,
    _z_candidates: &'a Vec<f32>,
    jacobians: &'a Vec<Vec6>,
    hessians: &'a Vec<Mat6>,
}

impl optim::State<Iso3, f32> for LMState {
    fn model(&self) -> &Iso3 {
        &self.data.model
    }
    fn energy(&self) -> f32 {
        self.data.energy
    }
}

impl<'a> Optimizer<Obs<'a>, LMState, Vec6, Iso3, PreEval, LMPartialState, f32> for LMOptimizer {
    fn initial_energy() -> f32 {
        f32::INFINITY
    }

    fn compute_step(state: &LMState) -> Option<Vec6> {
        let mut hessian = state.data.hessian.clone();
        hessian.m11 = (1.0 + state.lm_coef) * hessian.m11;
        hessian.m22 = (1.0 + state.lm_coef) * hessian.m22;
        hessian.m33 = (1.0 + state.lm_coef) * hessian.m33;
        hessian.m44 = (1.0 + state.lm_coef) * hessian.m44;
        hessian.m55 = (1.0 + state.lm_coef) * hessian.m55;
        hessian.m66 = (1.0 + state.lm_coef) * hessian.m66;
        hessian.cholesky().map(|ch| ch.solve(&state.data.gradient))
    }

    fn apply_step(delta: Vec6, model: &Iso3) -> Iso3 {
        let delta_warp = se3::exp(se3::from_vector(delta));
        let mut not_normalized = model * delta_warp.inverse();
        not_normalized.rotation = re_normalize(not_normalized.rotation);
        not_normalized
    }

    fn pre_eval(obs: &Obs, model: &Iso3) -> PreEval {
        let mut inside_indices = Vec::new();
        let mut residuals = Vec::new();
        let mut energy = 0.0;
        for (idx, &(x, y)) in obs.coordinates.iter().enumerate() {
            let _z = obs._z_candidates[idx];
            // check if warp(x,y) is inside the image
            let (u, v) = warp(model, x as f32, y as f32, _z, &obs.intrinsics);
            if let Some(im) = interpolate(u, v, &obs.image) {
                // precompute residuals and energy
                let tmp = obs.template[(y, x)];
                let r = im - tmp as f32;
                energy = energy + r * r;
                residuals.push(r);
                inside_indices.push(idx); // keep only inside points
            }
        }
        energy = energy / residuals.len() as f32;
        (inside_indices, residuals, energy)
    }

    fn eval(obs: &Obs, energy: f32, pre_eval: PreEval, model: Iso3) -> LMPartialState {
        let (inside_indices, residuals, new_energy) = pre_eval;
        if new_energy > energy {
            Err(new_energy)
        } else {
            let mut gradient = Vec6::zeros();
            let mut hessian = Mat6::zeros();
            for (i, idx) in inside_indices.iter().enumerate() {
                let jac = obs.jacobians[*idx];
                let hes = obs.hessians[*idx];
                let r = residuals[i];
                gradient = gradient + jac * r;
                hessian = hessian + hes;
            }
            Ok(LMData {
                hessian,
                gradient,
                energy: new_energy,
                model,
            })
        }
    }

    fn stop_criterion(
        nb_iter: usize,
        s0: LMState,
        s1: LMPartialState,
    ) -> (LMState, optim::Continue) {
        let too_many_iterations = nb_iter > 20;
        match (s1, too_many_iterations) {
            // Max number of iterations reached:
            (Err(_), true) => (s0, optim::Continue::Stop),
            (Ok(data), true) => {
                // eprintln!("Energy: {}", data.energy);
                // eprintln!("Gradient norm: {}", data.gradient.norm());
                let kept_state = LMState {
                    lm_coef: s0.lm_coef, // does not matter actually
                    data: data,
                };
                (kept_state, optim::Continue::Stop)
            }
            // Can continue to iterate:
            (Err(_energy), false) => {
                // eprintln!("\t back from: {}", energy);
                let mut kept_state = s0;
                kept_state.lm_coef = 10.0 * kept_state.lm_coef;
                (kept_state, optim::Continue::Forward)
            }
            (Ok(data), false) => {
                let d_energy = s0.data.energy - data.energy;
                let _gradient_norm = data.gradient.norm();
                // 1.0 is totally empiric here
                let continuation = if d_energy > 1.0 {
                    optim::Continue::Forward
                } else {
                    optim::Continue::Stop
                };
                // eprintln!("Energy: {}", data.energy);
                // eprintln!("Gradient norm: {}", gradient_norm);
                let kept_state = LMState {
                    lm_coef: 0.1 * s0.lm_coef,
                    data: data,
                };
                (kept_state, continuation)
            }
        }
    } // fn stop_criterion
} // impl Optimizer<...> for LMOptimizer

// Helper ######################################################################

// fn angle(uq: UnitQuaternion<f32>) -> f32 {
//     let w = uq.into_inner().scalar();
//     2.0 * uq.into_inner().vector().norm().atan2(w)
// }

fn re_normalize(uq: UnitQuaternion<f32>) -> UnitQuaternion<f32> {
    let q = uq.into_inner();
    let sq_norm = q.norm_squared();
    UnitQuaternion::new_unchecked(0.5 * (3.0 - sq_norm) * q)
}

fn im_gradient(im: &DMatrix<u8>) -> (DMatrix<i16>, DMatrix<i16>) {
    let (nb_rows, nb_cols) = im.shape();
    let top = im.slice((0, 1), (nb_rows - 2, nb_cols - 2));
    let bottom = im.slice((2, 1), (nb_rows - 2, nb_cols - 2));
    let left = im.slice((1, 0), (nb_rows - 2, nb_cols - 2));
    let right = im.slice((1, 2), (nb_rows - 2, nb_cols - 2));
    let mut grad_x = DMatrix::zeros(nb_rows, nb_cols);
    let mut grad_y = DMatrix::zeros(nb_rows, nb_cols);
    let mut grad_x_inner = grad_x.slice_mut((1, 1), (nb_rows - 2, nb_cols - 2));
    let mut grad_y_inner = grad_y.slice_mut((1, 1), (nb_rows - 2, nb_cols - 2));
    for j in 0..nb_cols - 2 {
        for i in 0..nb_rows - 2 {
            grad_x_inner[(i, j)] = (right[(i, j)] as i16 - left[(i, j)] as i16) / 2;
            grad_y_inner[(i, j)] = (bottom[(i, j)] as i16 - top[(i, j)] as i16) / 2;
        }
    }
    (grad_x, grad_y)
}

fn grad_squared_norm(grad_x: &DMatrix<i16>, grad_y: &DMatrix<i16>) -> DMatrix<u16> {
    grad_x.zip_map(grad_y, |gx, gy| {
        let gx = gx as i32;
        let gy = gy as i32;
        (gx * gx + gy * gy) as u16
    })
}

fn extract_z(idepth_mat: &DMatrix<InverseDepth>) -> (Vec<(usize, usize)>, Vec<f32>) {
    let mut u = 0;
    let mut v = 0;
    let mut coordinates = Vec::new();
    let mut _z_vec = Vec::new();
    let (nb_rows, _) = idepth_mat.shape();
    for idepth in idepth_mat.iter() {
        if let &InverseDepth::WithVariance(_z, _) = idepth {
            coordinates.push((u, v));
            _z_vec.push(_z);
        }
        v = v + 1;
        if v >= nb_rows {
            u = u + 1;
            v = 0;
        }
    }
    (coordinates, _z_vec)
}

fn warp_jacobians(
    intrinsics: &Intrinsics,
    coordinates: &Vec<(usize, usize)>,
    _z_candidates: &Vec<f32>,
    grad_x: &DMatrix<i16>,
    grad_y: &DMatrix<i16>,
) -> Vec<Vec6> {
    // Bind intrinsics to shorter names
    let (cu, cv) = intrinsics.principal_point;
    let (su, sv) = intrinsics.scaling;
    let fu = su * intrinsics.focal_length;
    let fv = sv * intrinsics.focal_length;
    let s = intrinsics.skew;

    // Iterate on inverse depth candidates
    coordinates
        .iter()
        .zip(_z_candidates.iter())
        .map(|(&(u, v), &_z)| {
            let gu = grad_x[(v, u)] as f32;
            let gv = grad_y[(v, u)] as f32;
            warp_jacobian_at(gu, gv, u as f32, v as f32, _z, cu, cv, fu, fv, s)
        })
        .collect()
}

fn warp_jacobian_at(
    gu: f32,
    gv: f32,
    u: f32,
    v: f32,
    _z: f32,
    cu: f32,
    cv: f32,
    fu: f32,
    fv: f32,
    s: f32,
) -> Vec6 {
    // Intermediate computations
    let a = u - cu;
    let b = v - cv;
    let c = a * fv - s * b;
    let _fv = 1.0 / fv;
    let _fuv = 1.0 / (fu * fv);

    // Jacobian of the warp
    #[rustfmt::skip]
    let jac = Vec6::new(
        gu * _z * fu,                                        //
        _z * (gu * s + gv * fv),                             //  linear velocity terms
        -_z * (gu * a + gv * b),                             //  ___
        gu * (-a * b * _fv - s) + gv * (-b * b * _fv - fv),  //
        gu * (a * c * _fuv + fu) + gv * (b * c * _fuv),      //  angular velocity terms
        gu * (-fu * fu * b + s * c) * _fuv + gv * (c / fu),  //
    );
    jac
}

fn hessians_vec(jacobians: &Vec<Vec6>) -> Vec<Mat6> {
    jacobians.iter().map(|j| j * j.transpose()).collect()
}

fn warp(model: &Iso3, x: f32, y: f32, _z: f32, intrinsics: &Intrinsics) -> (f32, f32) {
    let x1 = intrinsics.back_project(Point2::new(x, y), 1.0 / _z);
    let x2 = model * x1;
    let uvz2 = intrinsics.project(x2);
    (uvz2.x / uvz2.z, uvz2.y / uvz2.z)
}

fn interpolate(x: f32, y: f32, image: &DMatrix<u8>) -> Option<f32> {
    let (height, width) = image.shape();
    let u = x.floor();
    let v = y.floor();
    if u >= 0.0 && u < (width - 2) as f32 && v >= 0.0 && v < (height - 2) as f32 {
        let u_0 = u as usize;
        let v_0 = v as usize;
        let u_1 = u_0 + 1;
        let v_1 = v_0 + 1;
        let vu_00 = image[(v_0, u_0)] as f32;
        let vu_10 = image[(v_1, u_0)] as f32;
        let vu_01 = image[(v_0, u_1)] as f32;
        let vu_11 = image[(v_1, u_1)] as f32;
        let a = x - u;
        let b = y - v;
        Some(
            (1.0 - b) * (1.0 - a) * vu_00
                + b * (1.0 - a) * vu_10
                + (1.0 - b) * a * vu_01
                + b * a * vu_11,
        )
    } else {
        None
    }
}
