extern crate computer_vision_rs as cv;
extern crate nalgebra as na;

use cv::dso::candidates::{self, BlockConfig, RecursiveConfig, RegionConfig};
use cv::icl_nuim;
use cv::view;

use na::DMatrix;
use std::fs;

const OUT_DIR: &str = "out/icip/candidates_sequence_dso/";

// #[allow(dead_code)]
fn main() {
    // Configuration.
    let nb_img = 1509;

    // Create output directory.
    fs::create_dir_all(OUT_DIR).unwrap();

    for id_img in 0..nb_img {
        // Load image as grayscale (u8) and depth map (u16).
        let (img, _) = icl_nuim::open_imgs(id_img).unwrap();
        let candidates = generate_candidates(&img);

        // Save full resolution inverse depth map on disk.
        view::candidates_on_image(&img, &candidates)
            .save(&format!("{}{}.png", OUT_DIR, id_img))
            .unwrap();
    }
}

fn generate_candidates(img: &DMatrix<u8>) -> DMatrix<bool> {
    // Compute pyramid of gradients (without first level).
    let gradients = grad_squared_norm(img).map(|g2| (g2 as f32).sqrt() as u16);

    // Choose candidates based on gradients norms.
    let region_config = RegionConfig {
        size: 32,
        threshold_coefs: (1.0, 3), // (2.0, 3) in dso and (1.0, 7) in ldso
    };
    let block_config = BlockConfig {
        base_size: 4,
        nb_levels: 3,
        threshold_factor: 0.5,
    };
    let recursive_config = RecursiveConfig {
        nb_iterations_left: 1,
        low_thresh: 0.8,
        high_thresh: 4.0,
        random_thresh: 1.1,
    };

    let candidate_points = candidates::select(
        &gradients,
        region_config,
        block_config,
        recursive_config,
        2000,
    );
    // let final_nb_candidates = candidate_points.fold(0, |sum, x| if x { sum + 1 } else { sum });
    // println!("final_nb_candidates: {}", final_nb_candidates);

    candidate_points
}

// Helper ######################################################################

fn grad_squared_norm(im: &DMatrix<u8>) -> DMatrix<u16> {
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
