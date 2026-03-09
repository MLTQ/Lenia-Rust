use ndarray::{Array2, ArrayView2, Zip};
use rand::Rng;
use std::f64::consts::PI;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrowthFuncType {
    Polynomial = 0,
    Exponential = 1,
    Step = 2,
}

impl GrowthFuncType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Polynomial => "POLYNOMIAL",
            Self::Exponential => "EXPONENTIAL",
            Self::Step => "STEP",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelMode {
    CenteredGaussian,
    GaussianRings,
}

impl KernelMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CenteredGaussian => "CENTERED_GAUSSIAN",
            Self::GaussianRings => "GAUSSIAN_RINGS",
        }
    }
}

#[derive(Clone, Debug)]
pub struct LeniaParams {
    pub kernel_mode: KernelMode,
    pub kernel_size: usize,
    pub num_peaks: usize,
    pub betas: Vec<f64>,
    pub mu: f64,
    pub sigma: f64,
    pub dt: f64,
    pub growth_func_type: GrowthFuncType,
}

impl Default for LeniaParams {
    fn default() -> Self {
        Self {
            kernel_mode: KernelMode::CenteredGaussian,
            kernel_size: 21,
            num_peaks: 2,
            betas: vec![1.0, 5.0],
            mu: 1.01,
            sigma: 0.14,
            dt: 0.101,
            growth_func_type: GrowthFuncType::Exponential,
        }
    }
}

impl LeniaParams {
    pub fn gaussian_rings_preset() -> Self {
        Self {
            kernel_mode: KernelMode::GaussianRings,
            kernel_size: 39,
            num_peaks: 3,
            betas: vec![4.5, 2.0, 1.0],
            mu: 1.0,
            sigma: 0.18,
            dt: 0.04,
            growth_func_type: GrowthFuncType::Exponential,
        }
    }

    pub fn normalized_betas(&self) -> Vec<f64> {
        let peak_count = self.num_peaks.max(1);
        let mut betas = if self.betas.is_empty() {
            vec![1.0]
        } else {
            self.betas.clone()
        };

        if betas.len() < peak_count {
            let fill = *betas.last().unwrap_or(&1.0);
            betas.resize(peak_count, fill);
        } else {
            betas.truncate(peak_count);
        }

        for (index, beta) in betas.iter_mut().enumerate() {
            *beta = match self.kernel_mode {
                KernelMode::CenteredGaussian => beta.max(0.0001),
                KernelMode::GaussianRings if index == 0 => beta.max(0.0001),
                KernelMode::GaussianRings => beta.max(0.0),
            };
        }

        betas
    }
}

pub fn random_world(rows: usize, cols: usize) -> Array2<f64> {
    let mut rng = rand::thread_rng();
    let mut output = Array2::<f64>::zeros((rows, cols));
    for value in &mut output {
        *value = rng.gen::<f64>();
    }
    output
}

pub fn run_step(input: &Array2<f64>, params: &LeniaParams) -> Array2<f64> {
    let kernel = generate_kernel(params);
    let convolution = convolve2d_periodic(&input.view(), &kernel);
    let growth_mapped = apply_growth_mapping(
        &convolution,
        params.mu,
        params.sigma,
        params.growth_func_type,
    );
    update_world_array(input, &growth_mapped, params.dt)
}

pub fn apply_circular_brush(
    world: &mut Array2<f64>,
    center_col: usize,
    center_row: usize,
    brush_radius: usize,
    delta: f64,
) {
    let radius_sq = (brush_radius as isize) * (brush_radius as isize);
    let row_max = world.nrows() as isize;
    let col_max = world.ncols() as isize;

    for row_offset in -(brush_radius as isize)..=(brush_radius as isize) {
        for col_offset in -(brush_radius as isize)..=(brush_radius as isize) {
            if row_offset * row_offset + col_offset * col_offset > radius_sq {
                continue;
            }

            let row = center_row as isize + row_offset;
            let col = center_col as isize + col_offset;

            if row < 0 || col < 0 || row >= row_max || col >= col_max {
                continue;
            }

            let cell = &mut world[(row as usize, col as usize)];
            *cell = (*cell + delta).clamp(0.0, 1.0);
        }
    }
}

pub fn stamp_gaussian_blob(
    world: &mut Array2<f64>,
    center_col: usize,
    center_row: usize,
    size: usize,
    amplitude: f64,
    mu: f64,
    sigma: f64,
) {
    let blob_size = size.max(1);
    let half = (blob_size as isize) / 2;
    let sigma = sigma.max(0.0001);
    let row_max = world.nrows() as isize;
    let col_max = world.ncols() as isize;

    for local_row in 0..blob_size {
        for local_col in 0..blob_size {
            let row = center_row as isize + local_row as isize - half;
            let col = center_col as isize + local_col as isize - half;

            if row < 0 || col < 0 || row >= row_max || col >= col_max {
                continue;
            }

            let normalized_row = if blob_size == 1 {
                0.0
            } else {
                2.0 * local_row as f64 / (blob_size - 1) as f64 - 1.0
            };
            let normalized_col = if blob_size == 1 {
                0.0
            } else {
                2.0 * local_col as f64 / (blob_size - 1) as f64 - 1.0
            };
            let distance =
                (normalized_col * normalized_col + normalized_row * normalized_row).sqrt();
            let amount = amplitude * (-((distance - mu).powi(2)) / (2.0 * sigma * sigma)).exp();

            let cell = &mut world[(row as usize, col as usize)];
            *cell = (*cell + amount).clamp(0.0, 1.0);
        }
    }
}

fn wrap_index(index: isize, size: usize) -> usize {
    index.rem_euclid(size as isize) as usize
}

fn convolve2d_periodic(input: &ArrayView2<f64>, kernel: &Array2<f64>) -> Array2<f64> {
    let (rows, cols) = input.dim();
    let (kernel_rows, kernel_cols) = kernel.dim();
    let row_pad = (kernel_rows / 2) as isize;
    let col_pad = (kernel_cols / 2) as isize;
    let mut output = Array2::<f64>::zeros((rows, cols));

    for row in 0..rows {
        for col in 0..cols {
            let mut sum = 0.0;
            for kernel_row in 0..kernel_rows {
                for kernel_col in 0..kernel_cols {
                    let source_row = wrap_index(row as isize + kernel_row as isize - row_pad, rows);
                    let source_col = wrap_index(col as isize + kernel_col as isize - col_pad, cols);
                    sum += input[(source_row, source_col)] * kernel[(kernel_row, kernel_col)];
                }
            }
            output[(row, col)] = sum;
        }
    }

    output
}

pub fn generate_kernel(params: &LeniaParams) -> Array2<f64> {
    match params.kernel_mode {
        KernelMode::CenteredGaussian => generate_centered_gaussian_kernel(params),
        KernelMode::GaussianRings => generate_gaussian_rings_kernel(params),
    }
}

fn generate_centered_gaussian_kernel(params: &LeniaParams) -> Array2<f64> {
    let kernel_size = params.kernel_size.max(1) | 1;
    let peak_count = params.num_peaks.max(1);
    let betas = params.normalized_betas();
    let center = (kernel_size - 1) as f64 / 2.0;
    let mut kernel = Array2::<f64>::zeros((kernel_size, kernel_size));

    for ((row, col), value) in kernel.indexed_iter_mut() {
        let dy = row as f64 - center;
        let dx = col as f64 - center;
        let distance = (dx * dx + dy * dy).sqrt();
        let mut sum = 0.0;

        for beta in betas.iter().take(peak_count) {
            sum += (-(distance * distance) / (2.0 * beta * beta)).exp();
        }

        *value = sum / (peak_count as f64 * 2.0 * PI);
    }

    kernel
}

fn generate_gaussian_rings_kernel(params: &LeniaParams) -> Array2<f64> {
    let kernel_size = params.kernel_size.max(1) | 1;
    let peak_count = params.num_peaks.max(1);
    let betas = params.normalized_betas();
    let center = (kernel_size - 1) as f64 / 2.0;
    let max_radius = center.max(1.0);
    let base_width = betas[0].max(0.5);
    let ring_count = peak_count.saturating_sub(1);
    let ring_spacing = if ring_count > 0 {
        max_radius / peak_count as f64
    } else {
        max_radius
    };
    let ring_width = (base_width * 0.2).max(0.6);
    let target_mass = generate_centered_gaussian_kernel(params).sum();
    let mut kernel = Array2::<f64>::zeros((kernel_size, kernel_size));

    for ((row, col), value) in kernel.indexed_iter_mut() {
        let dy = row as f64 - center;
        let dx = col as f64 - center;
        let distance = (dx * dx + dy * dy).sqrt();
        let mut value_acc = (-(distance * distance) / (2.0 * base_width * base_width)).exp();

        for (ring_index, ring_weight) in betas.iter().enumerate().take(peak_count).skip(1) {
            let ring_center = ring_index as f64 * ring_spacing;
            let ring_profile =
                (-((distance - ring_center).powi(2)) / (2.0 * ring_width * ring_width)).exp();
            value_acc += ring_weight * ring_profile;
        }

        *value = value_acc / (2.0 * PI);
    }

    let kernel_mass = kernel.sum();
    if kernel_mass > 0.0 && target_mass > 0.0 {
        let scale = target_mass / kernel_mass;
        kernel.mapv_inplace(|value| value * scale);
    }

    kernel
}

#[cfg(test)]
mod tests {
    use super::{generate_kernel, KernelMode, LeniaParams};

    #[test]
    fn gaussian_rings_kernel_has_multiple_radial_peaks_for_preset() {
        let params = LeniaParams::gaussian_rings_preset();
        let kernel = generate_kernel(&params);
        let center = (kernel.nrows() as f64 - 1.0) * 0.5;
        let max_radius = center.floor() as usize;
        let mut sums = vec![0.0; max_radius + 1];
        let mut counts = vec![0usize; max_radius + 1];

        for ((row, col), value) in kernel.indexed_iter() {
            let dy = row as f64 - center;
            let dx = col as f64 - center;
            let radius = (dx * dx + dy * dy).sqrt().round() as usize;
            if radius <= max_radius {
                sums[radius] += *value;
                counts[radius] += 1;
            }
        }

        let profile: Vec<f64> = sums
            .into_iter()
            .zip(counts)
            .map(|(sum, count)| if count > 0 { sum / count as f64 } else { 0.0 })
            .collect();
        let peak_count = profile
            .windows(3)
            .filter(|window| window[1] > window[0] && window[1] > window[2])
            .count();

        assert_eq!(params.kernel_mode, KernelMode::GaussianRings);
        assert!(
            peak_count >= 2,
            "expected multiple radial peaks, got {peak_count}"
        );
    }
}

fn apply_growth_mapping(
    input: &Array2<f64>,
    mu: f64,
    sigma: f64,
    growth_func_type: GrowthFuncType,
) -> Array2<f64> {
    let safe_sigma = sigma.max(0.0001);
    let mut output = Array2::<f64>::zeros(input.dim());

    Zip::from(&mut output)
        .and(input)
        .for_each(|output_value, &input_value| {
            let diff = input_value - mu;
            *output_value = match growth_func_type {
                GrowthFuncType::Polynomial => {
                    let term = 1.0 - (diff * diff / (9.0 * safe_sigma * safe_sigma));
                    term.max(0.0).powi(4) * 2.0 - 1.0
                }
                GrowthFuncType::Exponential => {
                    (-diff * diff / (2.0 * safe_sigma * safe_sigma)).exp() * 2.0 - 1.0
                }
                GrowthFuncType::Step => {
                    if diff.abs() <= safe_sigma {
                        2.0
                    } else {
                        -1.0
                    }
                }
            };
        });

    output
}

fn update_world_array(input: &Array2<f64>, growth: &Array2<f64>, dt: f64) -> Array2<f64> {
    let mut output = Array2::<f64>::zeros(input.dim());
    Zip::from(&mut output).and(input).and(growth).for_each(
        |output_value, &input_value, &growth_value| {
            *output_value = (input_value + growth_value * dt).clamp(0.0, 1.0);
        },
    );
    output
}
