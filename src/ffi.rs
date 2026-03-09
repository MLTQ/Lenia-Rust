use crate::lenia::{run_step, GrowthFuncType, KernelMode, LeniaParams};
use ndarray::{ArrayView2, ArrayViewMut2};
use std::os::raw::{c_double, c_int};

#[no_mangle]
pub extern "C" fn run_lenia(
    input_ptr: *const c_double,
    rows: c_int,
    cols: c_int,
    kernel_size: c_int,
    num_peaks: c_int,
    betas_ptr: *const c_double,
    mu: c_double,
    sigma: c_double,
    dt: c_double,
    growth_func_type: GrowthFuncType,
    output_ptr: *mut c_double,
) {
    if input_ptr.is_null()
        || output_ptr.is_null()
        || betas_ptr.is_null()
        || rows <= 0
        || cols <= 0
        || kernel_size <= 0
        || num_peaks <= 0
    {
        return;
    }

    let rows = rows as usize;
    let cols = cols as usize;
    let peak_count = num_peaks as usize;
    let input_array = unsafe { ArrayView2::from_shape_ptr((rows, cols), input_ptr) };
    let betas = unsafe { std::slice::from_raw_parts(betas_ptr, peak_count) }.to_vec();

    let params = LeniaParams {
        kernel_mode: KernelMode::CenteredGaussian,
        kernel_size: kernel_size as usize,
        num_peaks: peak_count,
        betas,
        mu,
        sigma,
        dt,
        growth_func_type,
    };

    let output_array = run_step(&input_array.to_owned(), &params);
    let mut output_array_view = unsafe { ArrayViewMut2::from_shape_ptr((rows, cols), output_ptr) };
    output_array_view.assign(&output_array);
}
