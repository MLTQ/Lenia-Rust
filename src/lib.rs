use ndarray::prelude::*;
use ndarray::{Array2, ArrayView2, Zip, ArrayViewMut2};
use std::os::raw::{c_int, c_double};
use std::f64::consts::PI;
//
// fn convolve2d(input: &ArrayView2<f64>, kernel: &Array2<f64>) -> Array2<f64> {
//     let (n, m) = input.dim();
//     let (k_n, k_m) = kernel.dim();
//     let (p_n, p_m) = (k_n / 2, k_m / 2);
//
//     let mut output = Array2::zeros((n, m));
//
//     for i in 0..n {
//         for j in 0..m {
//             let i_start = if i >= p_n { i - p_n } else { 0 };
//             let i_end = (i + p_n + 1).min(n);
//             let j_start = if j >= p_m { j - p_m } else { 0 };
//             let j_end = (j + p_m + 1).min(m);
//
//             let mut sum = 0.0;
//
//             for ki in i_start..i_end {
//                 for kj in j_start..j_end {
//                     let kernel_i = ki - i + p_n;
//                     let kernel_j = kj - j + p_m;
//                     sum += input[[ki, kj]] * kernel[[kernel_i, kernel_j]];
//                 }
//             }
//
//             output[[i, j]] = sum;
//         }
//     }
//
//     output
// }
fn convolve2d(input: &ArrayView2<f64>, kernel: &Array2<f64>) -> Array2<f64> {
    let (n, m) = input.dim();
    let (k_n, k_m) = kernel.dim();
    let (p_n, p_m) = (k_n / 2, k_m / 2);

    let mut output = Array2::zeros((n, m));

    for i in 0..n {
        for j in 0..m {

            let mut sum = 0.0;

            for kernel_i in 0..k_n {
                for kernel_j in 0..k_m {
                    let input_i = wrap_index(i + kernel_i - p_n, n);
                    let input_j = wrap_index(j + kernel_j - p_m, m);
                    sum += input[[input_i, input_j]] * kernel[[kernel_i, kernel_j]];
                }
            }

            output[[i, j]] = sum;
        }
    }

    output
}

fn wrap_index(index: usize, size: usize) -> usize {
    index.rem_euclid(size)
}


// Kernel parameters struct
#[derive(Debug)]
pub struct KernelParams {
    kernel_size: usize,
    num_peaks: usize,
    betas: Vec<f64>,
}

// Function to generate the kernel K based on given parameters
fn generate_kernel(params: &KernelParams) -> Array2<f64> {
    // Create an empty 2D array of size (kernel_size, kernel_size)
    let mut kernel = Array2::<f64>::zeros((params.kernel_size, params.kernel_size));

    // Calculate the center of the kernel
    let center = (params.kernel_size - 1) as f64 / 2.0;

    // Iterate through each element in the kernel array
    for ((i, j), value) in kernel.indexed_iter_mut() {
        // Calculate the radial distance from the center of the kernel to the current element
        let dx = i as f64 - center;
        let dy = j as f64 - center;
        let distance = (dx * dx + dy * dy).sqrt();

        // Initialize the element value to 0
        let mut element_value = 0.0;

        // Iterate through the peaks in the kernel function
        for peak_idx in 0..params.num_peaks {
            // Calculate the value of the Gaussian function for the current peak
            let beta = params.betas[peak_idx];
            let gaussian_value = (-(distance * distance) / (2.0 * beta * beta)).exp();

            // Add the Gaussian value to the element value
            element_value += gaussian_value;
        }

        // Normalize the element value by the number of peaks and a scaling factor
        let scaling_factor = 2.0 * PI;
        *value = element_value / (params.num_peaks as f64 * scaling_factor);
    }

    // Return the generated kernel
    kernel
}

#[repr(C)]
pub enum GrowthFuncType {
    Polynomial,
    Exponential,
    Step,
}

// Function to apply the growth mapping function G
fn apply_growth_mapping(input: &Array2<f64>, mu: f64, sigma: f64, growth_func_type: GrowthFuncType) -> Array2<f64> {
    // Create an output array with the same dimensions as the input array
    let mut output = Array2::<f64>::zeros(input.dim());

    // Iterate over each element of the input array
    Zip::from(&mut output).and(input).for_each(|output_value, &input_value| {
        // Compute the difference between the input value and mu (the growth center)
        let diff = input_value - mu;

        // Apply the selected growth function based on the growth center (mu) and growth width (sigma)
        *output_value = match growth_func_type {
            GrowthFuncType::Polynomial => {
                let term = 1.0 - (diff * diff / (9.0 * sigma * sigma));
                (term.max(0.0).powi(4) * 2.0) - 1.0
            },
            GrowthFuncType::Exponential => {
                (-diff * diff / (2.0 * sigma * sigma)).exp() * 2.0 - 1.0
            },
            GrowthFuncType::Step => {
                if diff.abs() <= sigma { 2.0 } else { -1.0 }
            },
        };
    });

    // Return the output array
    output
}

// Function to update the world array A
fn update_world_array(input: &Array2<f64>, conv: &Array2<f64>, dt: f64) -> Array2<f64> {
    // Create an output array with the same dimensions as the input array
    let mut output = Array2::<f64>::zeros(input.dim());

    // Iterate over each element of the input array and the corresponding element of the convolution array
    Zip::from(&mut output).and(input).and(conv).for_each(|output_value, &input_value, &conv_value| {
        // Add a small portion (dt) of the convolution value to the input value
        let updated_value = input_value + conv_value * dt;

        // Clip the updated value to the range [0, 1] and store it in the output array
        *output_value = updated_value.max(0.0).min(1.0);
    });

    // Return the updated output array
    output
}



#[no_mangle]
pub extern "C" fn run_lenia(
    input_ptr: *const c_double,
    rows: c_int,
    cols: c_int,
    kernel_size: c_int,
    num_peaks: c_int,
    betas_ptr: *const c_double, // Add betas_ptr parameter
    mu: c_double,
    sigma: c_double,
    dt: c_double,
    growth_func_type: GrowthFuncType,
    output_ptr: *mut c_double,
) {
    // Convert input_ptr to ndarray
    let input_array = unsafe {
        ArrayView2::from_shape_ptr((rows as usize, cols as usize), input_ptr)
    };

    // Convert betas_ptr to Vec<f64>
    let betas = unsafe {
        std::slice::from_raw_parts(betas_ptr, num_peaks as usize).to_vec()
    };

    // Generate kernel K
    let kernel_params = KernelParams {
        kernel_size: kernel_size as usize,
        num_peaks: num_peaks as usize,
        betas, // Pass betas to the kernel_params
    };
    let kernel = generate_kernel(&kernel_params);

    // Calculate the convolution K * A
    let conv = convolve2d(&input_array, &kernel);

    // Apply the growth mapping function G
    let growth_mapped = apply_growth_mapping(&conv, mu, sigma, growth_func_type);

    // Update the world array A
    let output_array = update_world_array(&input_array.to_owned(), &growth_mapped.to_owned(), dt); // Call to_owned() on growth_mapped

    // Write output to output_ptr
    let mut output_array_view = unsafe {
        ArrayViewMut2::from_shape_ptr((rows as usize, cols as usize), output_ptr)
    };
    output_array_view.assign(&output_array);
}
