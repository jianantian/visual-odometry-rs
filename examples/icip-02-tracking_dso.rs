extern crate computer_vision_rs as cv;
extern crate itertools;
extern crate nalgebra as na;

use cv::camera::{self, Camera, Intrinsics};
use cv::dso::candidates;
use cv::helper;
use cv::icl_nuim::{self, INTRINSICS};
use cv::inverse_depth::{self, InverseDepth};
use cv::multires;
use cv::optimization_bis::{Continue, Optimizer, State};
use cv::se3;
use itertools::izip;
use na::{DMatrix, Matrix6, Point2, Vector6};
use std::{error::Error, f32};

fn main() {
    let _ = run();
}

fn run() -> Result<(), Box<Error>> {
    // Main parameters
    let tmp_id = 40;
    let img_id = 45;
    let nb_levels = 6;

    // Preload all extrinsic camera parameters of the trajectory
    let all_extrinsics = icl_nuim::read_extrinsics("data/trajectory-gt.txt")?;

    // Read the template image
    let (tmp, depth) = icl_nuim::open_imgs(tmp_id)?;
    let tmp_multires = multires::mean_pyramid(nb_levels, tmp);
    let tmp_cam_multires =
        Camera::new(INTRINSICS, all_extrinsics[tmp_id - 1].clone()).multi_res(nb_levels);

    // Read the transformed image
    let (img, _) = icl_nuim::open_imgs(img_id)?;
    let img_multires = multires::mean_pyramid(nb_levels, img);

    // Precompute template gradients
    let mut grad_multires = multires::gradients_xy(&tmp_multires);
    grad_multires.insert(0, im_gradient(&tmp_multires[0]));

    // Compute gradients norm of the template.
    let gradients = grad_squared_norm_dso(&tmp_multires[0]).map(|g2| (g2 as f32).sqrt() as u16);

    // Precompute candidate points
    let candidates = candidates::select(
        &gradients,
        candidates::DEFAULT_REGION_CONFIG,
        candidates::DEFAULT_BLOCK_CONFIG,
        candidates::DEFAULT_RECURSIVE_CONFIG,
        2000,
    );

    // Precompute inverse depth map
    let from_depth = |z| inverse_depth::from_depth(icl_nuim::DEPTH_SCALE, z);
    let idepth_candidates =
        helper::zip_mask_map(&depth, &candidates, InverseDepth::Unknown, from_depth);
    let fuse = |a, b, c, d| inverse_depth::fuse(a, b, c, d, inverse_depth::strategy_dso_mean);
    let idepth_multires = multires::limited_sequence(
        nb_levels,
        idepth_candidates,
        |m| m,
        |m| multires::halve(&m, fuse),
    );

    // Extract coordinates and idepth vectors
    let (coordinates_multires, _z_multires): (Vec<_>, Vec<_>) =
        idepth_multires.iter().map(extract_z).unzip();

    // Precompute the Jacobians
    let jacobians_multires: Vec<Vec<Vec6>> = izip!(
        &tmp_cam_multires,
        &coordinates_multires,
        &_z_multires,
        &grad_multires
    )
    .map(|(cam, coord, _z, (gx, gy))| warp_jacobians(&cam.intrinsics, coord, _z, gx, gy))
    .collect();

    // Precompute the Hessians
    let hessians_multires: Vec<_> = jacobians_multires.iter().map(hessians_vec).collect();

    // Multi-resolution optimization
    let mut model = Vec6::zeros();
    for lvl in (0..hessians_multires.len()).rev() {
        println!("---------------- Level {}:", lvl);
        let obs = Obs {
            intrinsics: &tmp_cam_multires[lvl].intrinsics,
            template: &tmp_multires[lvl],
            image: &img_multires[lvl],
            coordinates: &coordinates_multires[lvl],
            _z_candidates: &_z_multires[lvl],
            jacobians: &jacobians_multires[lvl],
            hessians: &hessians_multires[lvl],
        };
        let data = LMOptimizer::init(&obs, model).unwrap();
        let state = LMState { lm_coef: 0.1, data };
        let (state, _) = LMOptimizer::iterative(&obs, state);
        model = state.data.model;
    }

    println!(
        "Final motion: {}",
        se3::exp(se3::from_vector(model)).to_homogeneous()
    );

    // Compare with ground truth
    let img_cam_multires =
        Camera::new(INTRINSICS, all_extrinsics[img_id - 1].clone()).multi_res(nb_levels);
    let extrinsics_1 = tmp_cam_multires[0].extrinsics;
    let extrinsics_2 = img_cam_multires[0].extrinsics;
    let ext_gt = extrinsics_2.inverse() * extrinsics_1;
    println!("Ground truth: {}", ext_gt.to_homogeneous());
    Ok(())
}

struct LMOptimizer;
struct LMState {
    lm_coef: f32,
    data: LMData,
}
struct LMData {
    hessian: Mat6,
    gradient: Vec6,
    energy: f32,
    model: Vec6,
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
type Vec6 = Vector6<f32>;
type Mat6 = Matrix6<f32>;

impl State<Vec6, f32> for LMState {
    fn model(&self) -> &Vec6 {
        &self.data.model
    }
    fn energy(&self) -> f32 {
        self.data.energy
    }
}

impl<'a> Optimizer<Obs<'a>, LMState, Vec6, Vec6, PreEval, LMPartialState, f32> for LMOptimizer {
    fn initial_energy() -> f32 {
        f32::INFINITY
    }

    fn compute_step(state: &LMState) -> Vec6 {
        let mut hessian = state.data.hessian.clone();
        hessian.m11 = (1.0 + state.lm_coef) * hessian.m11;
        hessian.m22 = (1.0 + state.lm_coef) * hessian.m22;
        hessian.m33 = (1.0 + state.lm_coef) * hessian.m33;
        hessian.m44 = (1.0 + state.lm_coef) * hessian.m44;
        hessian.m55 = (1.0 + state.lm_coef) * hessian.m55;
        hessian.m66 = (1.0 + state.lm_coef) * hessian.m66;
        hessian.cholesky().unwrap().solve(&state.data.gradient)
    }

    fn apply_step(delta: Vec6, model: &Vec6) -> Vec6 {
        let delta_warp = se3::exp(se3::from_vector(delta));
        let old_warp = se3::exp(se3::from_vector(*model));
        let new_warp = old_warp * delta_warp.inverse();
        se3::to_vector(se3::log(new_warp))
    }

    fn pre_eval(obs: &Obs, model: &Vec6) -> PreEval {
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

    fn eval(obs: &Obs, energy: f32, pre_eval: PreEval, model: Vec6) -> LMPartialState {
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

    fn stop_criterion(nb_iter: usize, s0: LMState, s1: LMPartialState) -> (LMState, Continue) {
        let too_many_iterations = nb_iter > 20;
        match (s1, too_many_iterations) {
            // Max number of iterations reached:
            (Err(_), true) => (s0, Continue::Stop),
            (Ok(data), true) => {
                println!("Energy: {}", data.energy);
                println!("Gradient norm: {}", data.gradient.norm());
                let kept_state = LMState {
                    lm_coef: s0.lm_coef, // does not matter actually
                    data: data,
                };
                (kept_state, Continue::Stop)
            }
            // Can continue to iterate:
            (Err(energy), false) => {
                println!("\t back from: {}", energy);
                let mut kept_state = s0;
                kept_state.lm_coef = 10.0 * kept_state.lm_coef;
                (kept_state, Continue::Forward)
            }
            (Ok(data), false) => {
                let d_energy = s0.data.energy - data.energy;
                let gradient_norm = data.gradient.norm();
                // 1.0 is totally empiric here
                let continuation = if d_energy > 1.0 {
                    Continue::Forward
                } else {
                    Continue::Stop
                };
                println!("Energy: {}", data.energy);
                println!("Gradient norm: {}", gradient_norm);
                let kept_state = LMState {
                    lm_coef: 0.1 * s0.lm_coef,
                    data: data,
                };
                (kept_state, continuation)
            }
        }
    }
}

// Helper ######################################################################

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

fn grad_squared_norm_dso(im: &DMatrix<u8>) -> DMatrix<u16> {
    let (nb_rows, nb_cols) = im.shape();
    let top = im.slice((0, 1), (nb_rows - 2, nb_cols - 2));
    let bottom = im.slice((2, 1), (nb_rows - 2, nb_cols - 2));
    let left = im.slice((1, 0), (nb_rows - 2, nb_cols - 2));
    let right = im.slice((1, 2), (nb_rows - 2, nb_cols - 2));
    let mut grad_squared_norm_mat = DMatrix::zeros(nb_rows, nb_cols);
    let mut grad_inner = grad_squared_norm_mat.slice_mut((1, 1), (nb_rows - 2, nb_cols - 2));
    for j in 0..nb_cols - 2 {
        for i in 0..nb_rows - 2 {
            let gx = right[(i, j)] as i32 - left[(i, j)] as i32;
            let gy = bottom[(i, j)] as i32 - top[(i, j)] as i32;
            grad_inner[(i, j)] = ((gx * gx + gy * gy) / 4) as u16;
        }
    }
    grad_squared_norm_mat
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
    intrinsics: &camera::Intrinsics,
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

fn warp(model: &Vec6, x: f32, y: f32, _z: f32, intrinsics: &Intrinsics) -> (f32, f32) {
    let x1 = intrinsics.back_project(Point2::new(x, y), 1.0 / _z);
    let twist = se3::from_vector(*model);
    let x2 = se3::exp(twist) * x1;
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