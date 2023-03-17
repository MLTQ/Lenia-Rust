// lenia.rs

use ndarray::{Array2, ArrayView2, Ix2, s};
use std::f64::consts::PI;

pub struct KernelParams {
    pub size: usize,
    pub decay_constant: f64,
    pub decay_type: String,
    pub penalty_constant: f64,
}

pub fn apply_kernel(params: &KernelParams, input: &Array2<f64>) -> Array2<f64> {
    let mut result = Array2::zeros(input.dim());

    let KernelParams {
        size,
        decay_constant,
        decay_type,
        penalty_constant,
    } = params;

    let offset = size / 2;
    let kernel = generate_kernel(*size, *decay_constant, decay_type, params.penalty_constant);

    for ((i, j), value) in input.indexed_iter() {
        let submat = Array2::from_shape_fn((*size, *size), |(y, x)| {
            let wrapped_i = ((i + y + input.dim().0 - offset) % input.dim().0) as usize;
            let wrapped_j = ((j + x + input.dim().1 - offset) % input.dim().1) as usize;
            input[[wrapped_i, wrapped_j]]
        });

        let updated_value = submat.dot(&kernel);

        result[[i, j]] = *value + updated_value[[offset, offset]];

    }

    result
}





fn generate_kernel(size: usize, decay_constant: f64, decay_type: &str, penalty_constant: f64) -> Array2<f64> {
    let mut kernel = Array2::zeros((size, size));
    let center = (size / 2) as f64;
    //let penalty_constant = 1; // You can adjust this constant to control the strength of the penalty

    for ((i, j), value) in kernel.indexed_iter_mut() {
        let distance = ((i as f64 - center).powi(2) + (j as f64 - center).powi(2)).sqrt();
        let penalty = penalty_constant * distance.powi(2);

        *value = match decay_type {
            "exponential" => (-decay_constant * distance).exp() - penalty,
            "linear" | _ => 1.0 - decay_constant * distance - penalty,
        };
    }

    kernel
}
