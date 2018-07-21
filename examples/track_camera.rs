extern crate computer_vision_rs as cv;
extern crate image;
extern crate nalgebra;

use cv::camera::{Extrinsics, Intrinsics};
use cv::candidates;
use cv::helper;
use cv::icl_nuim;
use cv::inverse_depth::{self, InverseDepth};
use cv::multires;

use nalgebra::{DMatrix, Isometry3, Point2, Vector6};

fn main() {
    let all_extrinsics = Extrinsics::read_from_tum_file("data/trajectory-gt.txt").unwrap();
    let (multires_camera_1, multires_img_1, depth_1) =
        icl_nuim::prepare_data(1, &all_extrinsics).unwrap();
    let (multires_camera_2, multires_img_2, _) =
        icl_nuim::prepare_data(600, &all_extrinsics).unwrap();
    let multires_gradients_1_norm = multires::gradients(&multires_img_1);
    let multires_gradients_2_xy = multires::gradients_xy(&multires_img_2);
    let candidates = candidates::select(&multires_gradients_1_norm)
        .pop()
        .unwrap();
    let fuse =
        |a, b, c, d| inverse_depth::fuse(a, b, c, d, inverse_depth::strategy_statistically_similar);
    let half_res_depth = multires::halve(&depth_1.map(inverse_depth::from_depth), fuse).unwrap();
    let idepth_candidates = helper::zip_mask_map(
        &half_res_depth,
        &candidates,
        InverseDepth::Unknown,
        |idepth| idepth,
    );
    let multires_idepth = multires::pyramid_with_max_n_levels(
        5,
        idepth_candidates,
        |mat| mat,
        |mat| multires::halve(&mat, fuse),
    );
    let level = 2;
    let cam_1 = &multires_camera_1[level];
    let cam_2 = &multires_camera_2[level];
    let intrinsics = &cam_2.intrinsics;
    let img_1 = &multires_img_1[level];
    let img_2 = &multires_img_2[level];
    let idepth_map = &multires_idepth[level - 1];
    let (gx_2, gy_2) = &multires_gradients_2_xy[level - 1];
    let motion = track(intrinsics, idepth_map, img_1, img_2, gx_2, gy_2);
    println!("Computed motion: {}", motion);
    let iso_1 = Isometry3::from_parts(cam_1.extrinsics.translation, cam_1.extrinsics.rotation);
    let iso_2 = Isometry3::from_parts(cam_2.extrinsics.translation, cam_2.extrinsics.rotation);
    println!("Ground truth motion: {}", iso_2.inverse() * iso_1);
}

fn track(
    intrinsics: &Intrinsics,
    idepth_map: &DMatrix<InverseDepth>,
    img_1: &DMatrix<u8>,
    img_2: &DMatrix<u8>,
    gx_2: &DMatrix<i16>,
    gy_2: &DMatrix<i16>,
) -> Isometry3<Float> {
    // * For each pyramid level (starting with lowest resolution):
    //     * (1) Compute the smallest residual energy threshold, such that more than 40% residuals of points ar under this threshold.
    //     * (2) Compute and save all residuals (the energy functional)
    //     * While the energy decreases fast enough and we are under the limit of iterations:
    //         * (3) Compute Jacobians for each point
    //         * (4) Accumulate Jacobians products in accumulated Hessian
    //         * (5) Solve the step computation using Levenberg-Marquardt dampening instead of the Gauss-Newton
    //         * (6) Update (recompute) the residuals
    let (nrows, ncols) = idepth_map.shape();
    let mut motion = Isometry3::identity();
    let (f, (sx, sy)) = (intrinsics.focal_length, intrinsics.scaling);
    let focale = Focale(f * sx, f * sy);
    let mut all_jacobians = Vec::new();
    let mut nb_candidates = 0;
    let mut nb_in_frame = 0;
    // TODO: Convert the identity motion to initial se3 element
    // to start the Gauss-Newton iterations.
    for (index, idepth_enum) in idepth_map.iter().enumerate() {
        if let InverseDepth::WithVariance(idepth, _variance) = idepth_enum {
            nb_candidates += 1;
            let (col, row) = helper::div_rem(index, nrows);
            let point = Point2::new(col as f32, row as f32);
            let (reprojected, new_idepth) = reproject(point, *idepth, intrinsics, &motion);
            if helper::in_image_bounds((reprojected[0], reprojected[1]), (nrows, ncols)) {
                nb_in_frame += 1;
                let jacobian = jacobian_at(reprojected, new_idepth, img_2, gx_2, gy_2, &focale);
                // TODO: also compute the residual because it is needed
                // for Gauss-Newton step computation.
                // Below is the code from gauss_newton_2d example:
                // let step = |a, b| jacobian(a, b).svd(true, true).solve(&res(a, b), EPSILON);
                all_jacobians.push(jacobian);
            }
        }
    }
    // TODO: Convert the estimated se3 into Isometry to compare it.
    println!("Image resolution: {} x {}", ncols, nrows);
    println!("Nb candidates: {}", nb_candidates);
    println!("Still in frame: {}", nb_in_frame);
    motion
}

fn jacobian_at(
    point: Point2<Float>,
    idepth: Float,
    img: &DMatrix<u8>,
    gradient_x: &DMatrix<i16>,
    gradient_y: &DMatrix<i16>,
    focale: &Focale,
) -> Jacobian {
    let ((u, v), (a, b, c, d)) = linear_interpolator(point);
    let img_xy = a * img[(v, u)] as f32 + b * img[(v + 1, u)] as f32 + c * img[(v, u + 1)] as f32
        + d * img[(v + 1, u + 1)] as f32;
    let gx2_xy = a * gradient_x[(v, u)] as f32 + b * gradient_x[(v + 1, u)] as f32
        + c * gradient_x[(v, u + 1)] as f32
        + d * gradient_x[(v + 1, u + 1)] as f32;
    let gy2_xy = a * gradient_y[(v, u)] as f32 + b * gradient_y[(v + 1, u)] as f32
        + c * gradient_y[(v, u + 1)] as f32
        + d * gradient_y[(v + 1, u + 1)] as f32;
    let gradient = Gradient(gx2_xy, gy2_xy);
    compute_jacobian(point, idepth, focale, gradient)
}

fn linear_interpolator(
    coordinates: Point2<Float>,
) -> ((usize, usize), (Float, Float, Float, Float)) {
    let x = coordinates[0];
    let y = coordinates[1];
    let u = x.floor() as usize;
    let v = y.floor() as usize;
    let a = x - u as f32;
    let b = y - v as f32;
    let _a = 1.0 - a;
    let _b = 1.0 - b;
    ((u, v), (_a * _b, _a * b, a * _b, a * b))
}

// Step (1) is done by multiplying by 2 the threshold and recomputing the residuals every time (max 5 times) which seems very ineficient. But maybe it hapens rarely if the default threshold already allows more than 40% of points to have lower residuals.
//
// Step (2) is done with the function `CoarseTracker::calcRes()` (https://github.com/JakobEngel/dso/blob/master/src/FullSystem/CoarseTracker.cpp#L358). I'll summarize it as follows:

fn reproject(
    point: Point2<Float>,
    idepth: Float,
    intrinsics: &Intrinsics,
    motion: &Isometry3<Float>,
) -> (Point2<Float>, Float) {
    // * For each point of the current level:
    //     * (2.1) Compute and save the back-projected position and depth in the new frame (if not out of frame)
    //     * (2.2) Interpolate the color at this position
    //     * (2.3) Interpolate and save the gradients at this position (useful later for (3))
    //     * (2.4) Compute residuals with a Huber norm
    let homogeneous = intrinsics.project(motion * intrinsics.back_project(point, 1.0 / idepth));
    let new_idepth = 1.0 / homogeneous[2];
    let new_point = Point2::new(homogeneous[0] * new_idepth, homogeneous[1] * new_idepth);
    (new_point, new_idepth)
}

type Float = f32;
struct Focale(Float, Float);
struct Gradient(Float, Float);
struct Jacobian(Vector6<Float>);

// Step (3) and (4) are done in the function `CoarseTracker::calcGSSSE()`. Now I guess "GS" holds for Gauss-Newton System or GradientS or something related to computing Jacobians. And the "SSE" part is a reference to SSE processor instructions (https://en.wikipedia.org/wiki/Streaming_SIMD_Extensions). Those enable a bit more efficiency of repeated computation, but it makes the code extremely obscure! If I ignore the SSE semantics in this, the code of this function is basically:

fn compute_jacobian(point: Point2<Float>, idepth: Float, f: &Focale, g: Gradient) -> Jacobian {
    // * For each point:
    //     * (3.1) Scale saved gradients with the focale: dx = fx * dx and dy = fy * dy
    //     * (3.2) Compute 8 Jacobians (6 geometric, 2 photometric)
    //         J0 = idepth * dx
    //         J1 = idepth * dy
    //         J2 = - ( idepth * ( u * dx + v * dy ) )
    //         J3 = - ( dx * u * v + dy * ( 1 + v^2 ) )
    //         J4 = u * v * dy + dx * (1 + u^2 )
    //         J5 = u * dy - v * dx
    //         J6 = a * ( b - refColor )
    //         J7 = -1
    //     * (4.1) Accumulate the products of Jacobians in an Hessian matrix (H_ij += (Ji * Jj)).
    let (u, v) = (point[0], point[1]);
    let uv = u * v;
    let Focale(fx, fy) = f;
    let Gradient(dx, dy) = g;
    let fdx = fx * dx;
    let fdy = fy * dy;
    let jacobian = Vector6::new(
        idepth * fdx,
        idepth * fdy,
        -(idepth * (u * fdx + v * fdy)),
        -(uv * fdx + (1.0 + v * v) * fdy),
        uv * fdy + (1.0 + u * u) * fdx,
        u * fdy - v * fdx,
    );

    // Mathematically, we could compute the jacobian like below,
    // but it's just less efficient to do so:
    //
    // let gradient_transposed = Vector2::new(fx * dx, fy * dy);
    // let partial_derivates = Matrix6x2::from_columns(&[
    //     Vector6::new(idepth, 0.0, -idepth * u, -uv, 1.0 + u * u, -v),
    //     Vector6::new(0.0, idepth, -idepth * v, -(1.0 + v * v), uv, u),
    // ]);
    // let jacobian = partial_derivates * gradient_transposed;

    Jacobian(jacobian)
}
