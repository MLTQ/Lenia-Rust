use crate::lenia::{
    apply_circular_brush, generate_kernel, random_world, run_step, stamp_gaussian_blob,
    GrowthFuncType, KernelCoreType, KernelMode, LeniaParams,
};
use crate::species::curated_species;
use eframe::egui::{self, Color32, ColorImage, Sense, TextureHandle, TextureOptions};
use ndarray::Array2;
use rand::Rng;

const WORLD_SIZE: usize = 256;
const MIN_WORLD_SIZE: usize = 32;
const MAX_WORLD_SIZE: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaintTool {
    DrawLife,
    Erase,
    PlaceFood,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorScale {
    Grayscale,
    Viridis,
    Plasma,
    Inferno,
    Ocean,
}

impl ColorScale {
    fn as_str(self) -> &'static str {
        match self {
            Self::Grayscale => "GRAYSCALE",
            Self::Viridis => "VIRIDIS",
            Self::Plasma => "PLASMA",
            Self::Inferno => "INFERNO",
            Self::Ocean => "OCEAN",
        }
    }

    fn color(self, value: f64) -> Color32 {
        let value = value.clamp(0.0, 1.0);
        match self {
            Self::Grayscale => {
                let level = (value * 255.0) as u8;
                Color32::from_rgb(level, level, level)
            }
            Self::Viridis => sample_gradient(
                value,
                &[
                    Color32::from_rgb(68, 1, 84),
                    Color32::from_rgb(59, 82, 139),
                    Color32::from_rgb(33, 145, 140),
                    Color32::from_rgb(94, 201, 98),
                    Color32::from_rgb(253, 231, 37),
                ],
            ),
            Self::Plasma => sample_gradient(
                value,
                &[
                    Color32::from_rgb(13, 8, 135),
                    Color32::from_rgb(126, 3, 168),
                    Color32::from_rgb(203, 71, 119),
                    Color32::from_rgb(248, 149, 64),
                    Color32::from_rgb(240, 249, 33),
                ],
            ),
            Self::Inferno => sample_gradient(
                value,
                &[
                    Color32::from_rgb(0, 0, 4),
                    Color32::from_rgb(87, 15, 109),
                    Color32::from_rgb(187, 55, 84),
                    Color32::from_rgb(249, 142, 8),
                    Color32::from_rgb(252, 255, 164),
                ],
            ),
            Self::Ocean => sample_gradient(
                value,
                &[
                    Color32::from_rgb(2, 14, 28),
                    Color32::from_rgb(0, 79, 122),
                    Color32::from_rgb(0, 150, 136),
                    Color32::from_rgb(120, 220, 232),
                    Color32::from_rgb(236, 253, 255),
                ],
            ),
        }
    }
}

#[derive(Clone, Debug)]
struct ExplorerSettings {
    trial_count: usize,
    steps_per_trial: usize,
    evaluation_size: usize,
    mutation_scale: f64,
    keep_top: usize,
}

impl Default for ExplorerSettings {
    fn default() -> Self {
        Self {
            trial_count: 6,
            steps_per_trial: 10,
            evaluation_size: 96,
            mutation_scale: 0.22,
            keep_top: 6,
        }
    }
}

#[derive(Clone, Debug)]
struct ExplorerCandidate {
    params: LeniaParams,
    score: f64,
    final_mean: f64,
    activity: f64,
    variance: f64,
}

fn sample_gradient(value: f64, colors: &[Color32]) -> Color32 {
    if colors.is_empty() {
        return Color32::BLACK;
    }
    if colors.len() == 1 {
        return colors[0];
    }

    let scaled = value.clamp(0.0, 1.0) * (colors.len() - 1) as f64;
    let index = scaled.floor() as usize;
    let next_index = (index + 1).min(colors.len() - 1);
    let t = scaled - index as f64;
    lerp_color(colors[index], colors[next_index], t)
}

fn lerp_color(a: Color32, b: Color32, t: f64) -> Color32 {
    let t = t.clamp(0.0, 1.0) as f32;
    let lerp = |start: u8, end: u8| -> u8 {
        (start as f32 + (end as f32 - start as f32) * t).round() as u8
    };

    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

fn values_to_color_image(
    values: impl Iterator<Item = f64>,
    width: usize,
    height: usize,
    color_scale: ColorScale,
    max_value: f64,
) -> ColorImage {
    let scale = max_value.max(1e-12);
    let mut rgb = Vec::with_capacity(width * height * 3);

    for value in values {
        let color = color_scale.color(value / scale);
        rgb.push(color.r());
        rgb.push(color.g());
        rgb.push(color.b());
    }

    ColorImage::from_rgb([width, height], &rgb)
}

fn mean_abs_difference(left: &Array2<f64>, right: &Array2<f64>) -> f64 {
    let mut total = 0.0;
    for (lhs, rhs) in left.iter().zip(right.iter()) {
        total += (*lhs - *rhs).abs();
    }
    total / left.len() as f64
}

fn mean_variance(world: &Array2<f64>, mean: f64) -> f64 {
    let mut total = 0.0;
    for value in world.iter() {
        total += (*value - mean).powi(2);
    }
    total / world.len() as f64
}

fn centered_resized_copy(world: &Array2<f64>, rows: usize, cols: usize) -> Array2<f64> {
    let rows = rows.clamp(MIN_WORLD_SIZE, MAX_WORLD_SIZE);
    let cols = cols.clamp(MIN_WORLD_SIZE, MAX_WORLD_SIZE);
    let mut resized = Array2::<f64>::zeros((rows, cols));
    let row_copy = rows.min(world.nrows());
    let col_copy = cols.min(world.ncols());
    let src_row_start = (world.nrows() - row_copy) / 2;
    let src_col_start = (world.ncols() - col_copy) / 2;
    let dst_row_start = (rows - row_copy) / 2;
    let dst_col_start = (cols - col_copy) / 2;

    for row in 0..row_copy {
        for col in 0..col_copy {
            resized[(dst_row_start + row, dst_col_start + col)] =
                world[(src_row_start + row, src_col_start + col)];
        }
    }

    resized
}

#[derive(Clone, Debug)]
struct FoodSettings {
    enabled: bool,
    randomize_each_refresh: bool,
    refresh_period: usize,
    source_count: usize,
    blob_size: usize,
    blob_amplitude: f64,
    blob_mu: f64,
    blob_sigma: f64,
    source_positions: Vec<(usize, usize)>,
}

impl Default for FoodSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            randomize_each_refresh: false,
            refresh_period: 10,
            source_count: 10,
            blob_size: 21,
            blob_amplitude: 0.1,
            blob_mu: 1.0,
            blob_sigma: 0.1,
            source_positions: Vec::new(),
        }
    }
}

pub struct LeniaApp {
    world: Array2<f64>,
    params: LeniaParams,
    selected_species_index: usize,
    color_scale: ColorScale,
    running: bool,
    steps_per_frame: usize,
    paint_tool: PaintTool,
    brush_radius: usize,
    brush_strength: f64,
    food: FoodSettings,
    frame_counter: usize,
    texture: Option<TextureHandle>,
    kernel_texture: Option<TextureHandle>,
    pending_world_rows: usize,
    pending_world_cols: usize,
    explorer: ExplorerSettings,
    explorer_results: Vec<ExplorerCandidate>,
}

impl LeniaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            world: random_world(WORLD_SIZE, WORLD_SIZE),
            params: LeniaParams::default(),
            selected_species_index: 0,
            color_scale: ColorScale::Grayscale,
            running: true,
            steps_per_frame: 1,
            paint_tool: PaintTool::DrawLife,
            brush_radius: 4,
            brush_strength: 0.2,
            food: FoodSettings::default(),
            frame_counter: 0,
            texture: None,
            kernel_texture: None,
            pending_world_rows: WORLD_SIZE,
            pending_world_cols: WORLD_SIZE,
            explorer: ExplorerSettings::default(),
            explorer_results: Vec::new(),
        };
        app.ensure_beta_count();
        app.regenerate_food_sources();
        app.apply_food_sources();
        app
    }

    fn ensure_beta_count(&mut self) {
        self.params.betas = self.params.normalized_betas();
    }

    fn apply_centered_gaussian_preset(&mut self) {
        self.params = LeniaParams::default();
        self.ensure_beta_count();
    }

    fn apply_gaussian_rings_preset(&mut self) {
        self.params = LeniaParams::gaussian_rings_preset();
        self.ensure_beta_count();
    }

    fn apply_selected_species(&mut self) {
        let Some(species) = curated_species().get(self.selected_species_index).copied() else {
            return;
        };
        let Some(loaded) = species.load() else {
            return;
        };

        self.params = loaded.params;
        self.world = loaded.world;
        self.pending_world_rows = self.world.nrows();
        self.pending_world_cols = self.world.ncols();
        self.food.enabled = false;
        self.food.source_positions.clear();
        self.frame_counter = 0;
        self.explorer_results.clear();
        self.ensure_beta_count();
    }

    fn resize_world(&mut self, rows: usize, cols: usize) {
        let rows = rows.clamp(MIN_WORLD_SIZE, MAX_WORLD_SIZE);
        let cols = cols.clamp(MIN_WORLD_SIZE, MAX_WORLD_SIZE);
        if rows == self.world.nrows() && cols == self.world.ncols() {
            return;
        }

        self.world = centered_resized_copy(&self.world, rows, cols);
        self.pending_world_rows = rows;
        self.pending_world_cols = cols;
        self.frame_counter = 0;
        self.explorer_results.clear();
        self.regenerate_food_sources();
    }

    fn run_explorer_search(&mut self) {
        let mut rng = rand::thread_rng();
        let base_world = centered_resized_copy(
            &self.world,
            self.explorer.evaluation_size,
            self.explorer.evaluation_size,
        );
        let base_params = self.params.clone();
        let mut candidates = Vec::with_capacity(self.explorer.trial_count);

        for _ in 0..self.explorer.trial_count {
            let candidate_params = self.mutated_params(&base_params, &mut rng);
            if let Some(candidate) = self.evaluate_candidate(&base_world, candidate_params) {
                candidates.push(candidate);
            }
        }

        candidates.sort_by(|left, right| right.score.total_cmp(&left.score));
        candidates.truncate(self.explorer.keep_top.max(1));
        self.explorer_results = candidates;
    }

    fn mutated_params<R: Rng>(&self, base: &LeniaParams, rng: &mut R) -> LeniaParams {
        let mut params = base.clone();

        if rng.gen_bool((0.2 * self.explorer.mutation_scale).clamp(0.0, 0.6)) {
            let delta = if rng.gen_bool(0.5) { 2isize } else { -2isize };
            let kernel_size = params.kernel_size as isize + delta;
            params.kernel_size = kernel_size.clamp(3, 99) as usize;
        }
        params.kernel_size |= 1;

        if rng.gen_bool((0.35 * self.explorer.mutation_scale).clamp(0.0, 0.75)) {
            let delta = if rng.gen_bool(0.5) { 1isize } else { -1isize };
            params.num_peaks = ((params.num_peaks as isize + delta).clamp(1, 8)) as usize;
        }

        params.betas = params.normalized_betas();
        for (index, beta) in params.betas.iter_mut().enumerate() {
            match params.kernel_mode {
                KernelMode::CenteredGaussian => {
                    let factor = (1.0 + rng.gen_range(-1.0..=1.0) * self.explorer.mutation_scale)
                        .clamp(0.2, 5.0);
                    *beta = (*beta * factor).clamp(0.01, 25.0);
                }
                KernelMode::GaussianRings if index == 0 => {
                    let factor = (1.0 + rng.gen_range(-1.0..=1.0) * self.explorer.mutation_scale)
                        .clamp(0.2, 4.0);
                    *beta = (*beta * factor).clamp(0.01, 25.0);
                }
                KernelMode::GaussianRings => {
                    let delta = rng.gen_range(-1.0..=1.0) * self.explorer.mutation_scale * 2.5;
                    *beta = (*beta + delta).clamp(0.0, 8.0);
                }
                KernelMode::LeniaBands => {
                    let delta = rng.gen_range(-1.0..=1.0) * self.explorer.mutation_scale * 0.35;
                    *beta = (*beta + delta).clamp(0.05, 2.0);
                }
            }
        }

        params.mu = (params.mu + rng.gen_range(-1.0..=1.0) * self.explorer.mutation_scale * 0.12)
            .clamp(0.0, 2.0);
        params.sigma = (params.sigma
            + rng.gen_range(-1.0..=1.0) * self.explorer.mutation_scale * 0.06)
            .clamp(0.01, 1.0);
        params.dt = (params.dt + rng.gen_range(-1.0..=1.0) * self.explorer.mutation_scale * 0.03)
            .clamp(0.001, 0.5);
        params
    }

    fn evaluate_candidate(
        &self,
        base_world: &Array2<f64>,
        params: LeniaParams,
    ) -> Option<ExplorerCandidate> {
        let mut world = base_world.clone();
        let initial_mean = world.sum() / world.len() as f64;
        let mut total_change = 0.0;

        for _ in 0..self.explorer.steps_per_trial {
            let next = run_step(&world, &params);
            total_change += mean_abs_difference(&world, &next);
            world = next;
        }

        let final_mean = world.sum() / world.len() as f64;
        if final_mean <= 1e-5 {
            return None;
        }

        let variance = mean_variance(&world, final_mean);
        let activity = total_change / self.explorer.steps_per_trial.max(1) as f64;
        let survival = if (0.01..0.95).contains(&final_mean) {
            1.0
        } else {
            0.0
        };
        let stability = (1.0 - ((final_mean - initial_mean).abs() / 0.35)).clamp(0.0, 1.0);
        let activity_score = (activity / 0.03).clamp(0.0, 1.5);
        let variance_score = (variance / 0.02).clamp(0.0, 1.5);
        let score = survival * 2.0 + stability * 0.75 + activity_score + variance_score;

        Some(ExplorerCandidate {
            params,
            score,
            final_mean,
            activity,
            variance,
        })
    }

    fn regenerate_food_sources(&mut self) {
        self.food.source_positions.clear();
        let mut rng = rand::thread_rng();
        for _ in 0..self.food.source_count {
            let col = rng.gen_range(0..self.world.ncols());
            let row = rng.gen_range(0..self.world.nrows());
            self.food.source_positions.push((col, row));
        }
    }

    fn place_food_at(&mut self, col: usize, row: usize) {
        stamp_gaussian_blob(
            &mut self.world,
            col,
            row,
            self.food.blob_size,
            self.food.blob_amplitude,
            self.food.blob_mu,
            self.food.blob_sigma,
        );
    }

    fn apply_food_sources(&mut self) {
        if !self.food.enabled {
            return;
        }

        if self.food.randomize_each_refresh {
            let mut rng = rand::thread_rng();
            for _ in 0..self.food.source_count {
                let col = rng.gen_range(0..self.world.ncols());
                let row = rng.gen_range(0..self.world.nrows());
                self.place_food_at(col, row);
            }
            return;
        }

        if self.food.source_positions.len() != self.food.source_count {
            self.regenerate_food_sources();
        }

        let positions = self.food.source_positions.clone();
        for (col, row) in positions {
            self.place_food_at(col, row);
        }
    }

    fn step_once(&mut self) {
        self.world = run_step(&self.world, &self.params);
        self.frame_counter += 1;

        if self.food.enabled
            && self.food.refresh_period > 0
            && self.frame_counter % self.food.refresh_period == 0
        {
            self.apply_food_sources();
        }
    }

    fn apply_tool(&mut self, col: usize, row: usize) {
        match self.paint_tool {
            PaintTool::DrawLife => {
                apply_circular_brush(
                    &mut self.world,
                    col,
                    row,
                    self.brush_radius,
                    self.brush_strength.abs(),
                );
            }
            PaintTool::Erase => {
                apply_circular_brush(
                    &mut self.world,
                    col,
                    row,
                    self.brush_radius,
                    -self.brush_strength.abs(),
                );
            }
            PaintTool::PlaceFood => self.place_food_at(col, row),
        }
    }

    fn world_to_image(&self) -> ColorImage {
        values_to_color_image(
            self.world.iter().copied(),
            self.world.ncols(),
            self.world.nrows(),
            self.color_scale,
            1.0,
        )
    }

    fn refresh_texture(&mut self, ctx: &egui::Context) {
        let image = self.world_to_image();
        if let Some(texture) = &mut self.texture {
            texture.set(image, TextureOptions::NEAREST);
        } else {
            self.texture = Some(ctx.load_texture("lenia-world", image, TextureOptions::NEAREST));
        }
    }

    fn kernel_to_image(&self) -> ColorImage {
        let kernel = generate_kernel(&self.params);
        let max_value = kernel.iter().copied().fold(0.0_f64, f64::max);
        values_to_color_image(
            kernel.iter().copied(),
            kernel.ncols(),
            kernel.nrows(),
            self.color_scale,
            max_value.max(1e-12),
        )
    }

    fn refresh_kernel_texture(&mut self, ctx: &egui::Context) {
        let image = self.kernel_to_image();
        if let Some(texture) = &mut self.kernel_texture {
            texture.set(image, TextureOptions::LINEAR);
        } else {
            self.kernel_texture =
                Some(ctx.load_texture("lenia-kernel", image, TextureOptions::LINEAR));
        }
    }

    fn kernel_radial_profile(&self) -> Vec<f64> {
        let kernel = generate_kernel(&self.params);
        let center_row = (kernel.nrows() as f64 - 1.0) * 0.5;
        let center_col = (kernel.ncols() as f64 - 1.0) * 0.5;
        let max_radius = center_row.min(center_col).floor() as usize;
        let mut sums = vec![0.0; max_radius + 1];
        let mut counts = vec![0usize; max_radius + 1];

        for ((row, col), value) in kernel.indexed_iter() {
            let dy = row as f64 - center_row;
            let dx = col as f64 - center_col;
            let radius = (dx * dx + dy * dy).sqrt().round() as usize;
            if radius <= max_radius {
                sums[radius] += *value;
                counts[radius] += 1;
            }
        }

        sums.into_iter()
            .zip(counts)
            .map(|(sum, count)| if count > 0 { sum / count as f64 } else { 0.0 })
            .collect()
    }

    fn draw_radial_kernel_plot(&self, ui: &mut egui::Ui, size: egui::Vec2) {
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        let profile = self.kernel_radial_profile();
        let max_value = profile.iter().copied().fold(0.0_f64, f64::max);

        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        );

        if profile.len() < 2 || max_value <= 0.0 {
            return;
        }

        let plot_rect = rect.shrink2(egui::vec2(10.0, 10.0));
        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), plot_rect.bottom()),
                egui::pos2(plot_rect.right(), plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, ui.visuals().weak_text_color()),
        );
        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), plot_rect.top()),
                egui::pos2(plot_rect.left(), plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, ui.visuals().weak_text_color()),
        );

        let points: Vec<egui::Pos2> = profile
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let x = if profile.len() == 1 {
                    plot_rect.left()
                } else {
                    egui::lerp(
                        plot_rect.left()..=plot_rect.right(),
                        index as f32 / (profile.len() - 1) as f32,
                    )
                };
                let y = egui::lerp(
                    plot_rect.bottom()..=plot_rect.top(),
                    (*value as f32 / max_value as f32).clamp(0.0, 1.0),
                );
                egui::pos2(x, y)
            })
            .collect();

        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
        ));
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Lenia Controls");
        ui.horizontal(|ui| {
            if ui
                .button(if self.running { "Pause" } else { "Play" })
                .clicked()
            {
                self.running = !self.running;
            }
            if ui.button("Step").clicked() {
                self.step_once();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Randomize").clicked() {
                self.world = random_world(self.world.nrows(), self.world.ncols());
                self.explorer_results.clear();
            }
            if ui.button("Clear").clicked() {
                self.world.fill(0.0);
                self.explorer_results.clear();
            }
        });
        ui.horizontal(|ui| {
            ui.label("field");
            ui.add(
                egui::DragValue::new(&mut self.pending_world_cols)
                    .range(MIN_WORLD_SIZE..=MAX_WORLD_SIZE)
                    .prefix("w "),
            );
            ui.add(
                egui::DragValue::new(&mut self.pending_world_rows)
                    .range(MIN_WORLD_SIZE..=MAX_WORLD_SIZE)
                    .prefix("h "),
            );
            if ui.button("Apply Size").clicked() {
                self.resize_world(self.pending_world_rows, self.pending_world_cols);
            }
        });
        egui::ComboBox::from_label("color_scale")
            .selected_text(self.color_scale.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.color_scale,
                    ColorScale::Grayscale,
                    ColorScale::Grayscale.as_str(),
                );
                ui.selectable_value(
                    &mut self.color_scale,
                    ColorScale::Viridis,
                    ColorScale::Viridis.as_str(),
                );
                ui.selectable_value(
                    &mut self.color_scale,
                    ColorScale::Plasma,
                    ColorScale::Plasma.as_str(),
                );
                ui.selectable_value(
                    &mut self.color_scale,
                    ColorScale::Inferno,
                    ColorScale::Inferno.as_str(),
                );
                ui.selectable_value(
                    &mut self.color_scale,
                    ColorScale::Ocean,
                    ColorScale::Ocean.as_str(),
                );
            });
        ui.add(egui::Slider::new(&mut self.steps_per_frame, 1..=8).text("steps/frame"));
        ui.label(format!(
            "Field: {}x{}  Mean population: {:.3}",
            self.world.ncols(),
            self.world.nrows(),
            self.world.sum() / (self.world.len() as f64)
        ));

        ui.separator();
        ui.collapsing("Species Library", |ui| {
            let species = curated_species();
            if species.is_empty() {
                ui.label("No species presets embedded.");
                return;
            }

            egui::ComboBox::from_label("species")
                .selected_text(
                    species[self.selected_species_index.min(species.len() - 1)].short_label(),
                )
                .show_ui(ui, |ui| {
                    for (index, preset) in species.iter().copied().enumerate() {
                        ui.selectable_value(
                            &mut self.selected_species_index,
                            index,
                            preset.short_label(),
                        );
                    }
                });

            let selected = species[self.selected_species_index.min(species.len() - 1)];
            ui.label(selected.detail_label());
            ui.label(
                "Loads the archived pattern into a recommended field and applies official R/T/b/m/s/kn/gn settings with the LENIA_BANDS kernel.",
            );
            ui.label("Field size is chosen automatically from the archived pattern and kernel radius.");
            if ui.button("Load Species").clicked() {
                self.apply_selected_species();
            }
        });

        ui.separator();
        ui.collapsing("Lenia Parameters", |ui| {
            let previous_mode = self.params.kernel_mode;
            egui::ComboBox::from_label("kernel_mode")
                .selected_text(self.params.kernel_mode.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.params.kernel_mode,
                        KernelMode::CenteredGaussian,
                        KernelMode::CenteredGaussian.as_str(),
                    );
                    ui.selectable_value(
                        &mut self.params.kernel_mode,
                        KernelMode::GaussianRings,
                        KernelMode::GaussianRings.as_str(),
                    );
                    ui.selectable_value(
                        &mut self.params.kernel_mode,
                        KernelMode::LeniaBands,
                        KernelMode::LeniaBands.as_str(),
                    );
                });
            if self.params.kernel_mode != previous_mode {
                self.ensure_beta_count();
            }

            if matches!(self.params.kernel_mode, KernelMode::LeniaBands) {
                egui::ComboBox::from_label("kernel_core_type")
                    .selected_text(self.params.kernel_core_type.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.params.kernel_core_type,
                            KernelCoreType::Polynomial,
                            KernelCoreType::Polynomial.as_str(),
                        );
                        ui.selectable_value(
                            &mut self.params.kernel_core_type,
                            KernelCoreType::Exponential,
                            KernelCoreType::Exponential.as_str(),
                        );
                        ui.selectable_value(
                            &mut self.params.kernel_core_type,
                            KernelCoreType::Step,
                            KernelCoreType::Step.as_str(),
                        );
                        ui.selectable_value(
                            &mut self.params.kernel_core_type,
                            KernelCoreType::Staircase,
                            KernelCoreType::Staircase.as_str(),
                        );
                    });
            }

            ui.horizontal(|ui| {
                if ui.button("Gaussian Preset").clicked() {
                    self.apply_centered_gaussian_preset();
                }
                if ui.button("Ring Preset").clicked() {
                    self.apply_gaussian_rings_preset();
                }
            });

            let mut kernel_size = self.params.kernel_size as u32;
            if ui
                .add(egui::Slider::new(&mut kernel_size, 3..=99).text("kernel_size"))
                .changed()
            {
                let adjusted = if kernel_size % 2 == 0 {
                    kernel_size + 1
                } else {
                    kernel_size
                };
                self.params.kernel_size = adjusted as usize;
            }

            if ui
                .add(egui::Slider::new(&mut self.params.num_peaks, 1..=8).text("num_peaks"))
                .changed()
            {
                self.ensure_beta_count();
            }

            self.ensure_beta_count();
            for (index, beta) in self.params.betas.iter_mut().enumerate() {
                let (prefix, range) = match self.params.kernel_mode {
                    KernelMode::CenteredGaussian => (format!("beta[{index}] "), 0.01..=25.0),
                    KernelMode::GaussianRings if index == 0 => {
                        ("core_width ".to_string(), 0.01..=25.0)
                    }
                    KernelMode::GaussianRings => {
                        (format!("ring[{index}] "), 0.0..=8.0)
                    }
                    KernelMode::LeniaBands => (format!("shell[{index}] "), 0.0..=2.0),
                };
                ui.add(
                    egui::DragValue::new(beta)
                        .speed(0.05)
                        .range(range)
                        .prefix(prefix),
                );
            }

            ui.label(match self.params.kernel_mode {
                KernelMode::CenteredGaussian => "Centered Gaussian: betas are Gaussian widths.",
                KernelMode::GaussianRings => {
                    "Gaussian Rings: beta[0] is the stable core width; later values boost outer rings."
                }
                KernelMode::LeniaBands => {
                    "Lenia Bands: shell weights follow the official segmented kernel; dt is 1/T."
                }
            });

            ui.add(
                egui::Slider::new(&mut self.params.mu, 0.0..=2.0)
                    .text("mu")
                    .step_by(0.001),
            );
            ui.add(
                egui::Slider::new(&mut self.params.sigma, 0.01..=1.0)
                    .text("sigma")
                    .step_by(0.001),
            );
            ui.add(
                egui::Slider::new(&mut self.params.dt, 0.001..=0.5)
                    .text("dt")
                    .step_by(0.001),
            );

            egui::ComboBox::from_label("growth_func_type")
                .selected_text(self.params.growth_func_type.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.params.growth_func_type,
                        GrowthFuncType::Polynomial,
                        GrowthFuncType::Polynomial.as_str(),
                    );
                    ui.selectable_value(
                        &mut self.params.growth_func_type,
                        GrowthFuncType::Exponential,
                        GrowthFuncType::Exponential.as_str(),
                    );
                    ui.selectable_value(
                        &mut self.params.growth_func_type,
                        GrowthFuncType::Step,
                        GrowthFuncType::Step.as_str(),
                    );
                });
        });

        ui.separator();
        ui.collapsing("Food", |ui| {
            ui.checkbox(&mut self.food.enabled, "Enable periodic food");
            if ui
                .checkbox(
                    &mut self.food.randomize_each_refresh,
                    "Randomize locations each refresh",
                )
                .changed()
                && !self.food.randomize_each_refresh
            {
                self.regenerate_food_sources();
            }

            ui.add(
                egui::Slider::new(&mut self.food.refresh_period, 1..=240).text("refresh period"),
            );
            if ui
                .add(egui::Slider::new(&mut self.food.source_count, 1..=64).text("food sources"))
                .changed()
                && !self.food.randomize_each_refresh
            {
                self.regenerate_food_sources();
            }
            ui.add(egui::Slider::new(&mut self.food.blob_size, 3..=63).text("blob size"));
            ui.add(
                egui::Slider::new(&mut self.food.blob_amplitude, 0.01..=1.0)
                    .text("blob amplitude")
                    .step_by(0.01),
            );
            ui.add(
                egui::Slider::new(&mut self.food.blob_mu, 0.0..=1.5)
                    .text("blob mu")
                    .step_by(0.01),
            );
            ui.add(
                egui::Slider::new(&mut self.food.blob_sigma, 0.01..=1.0)
                    .text("blob sigma")
                    .step_by(0.01),
            );

            ui.horizontal(|ui| {
                if ui.button("Seed food now").clicked() {
                    self.apply_food_sources();
                }
                if ui.button("Regenerate sources").clicked() {
                    self.regenerate_food_sources();
                }
            });
        });

        ui.separator();
        ui.collapsing("Draw", |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.paint_tool, PaintTool::DrawLife, "Draw Life");
                ui.selectable_value(&mut self.paint_tool, PaintTool::Erase, "Erase");
                ui.selectable_value(&mut self.paint_tool, PaintTool::PlaceFood, "Place Food");
            });
            ui.add(egui::Slider::new(&mut self.brush_radius, 1..=32).text("brush radius"));
            ui.add(
                egui::Slider::new(&mut self.brush_strength, 0.01..=1.0)
                    .text("brush strength")
                    .step_by(0.01),
            );
            ui.label("Click and drag on the simulation to paint.");
        });

        ui.separator();
        ui.collapsing("Explorer", |ui| {
            ui.label("Search nearby parameter space on a smaller centered copy of the field.");
            ui.add(egui::Slider::new(&mut self.explorer.trial_count, 2..=24).text("trials"));
            ui.add(
                egui::Slider::new(&mut self.explorer.steps_per_trial, 2..=48).text("steps/trial"),
            );
            ui.add(
                egui::Slider::new(&mut self.explorer.evaluation_size, 32..=256).text("eval size"),
            );
            ui.add(
                egui::Slider::new(&mut self.explorer.mutation_scale, 0.05..=1.0)
                    .text("mutation scale")
                    .step_by(0.01),
            );
            ui.add(egui::Slider::new(&mut self.explorer.keep_top, 1..=12).text("keep top"));
            if ui.button("Run Explorer").clicked() {
                self.run_explorer_search();
            }
            if self.explorer_results.is_empty() {
                ui.label("No results yet.");
            } else {
                let mut apply_result_index = None;
                for (index, candidate) in self.explorer_results.iter().enumerate() {
                    ui.separator();
                    ui.label(format!(
                        "#{index} score {:.2}  pop {:.3}  act {:.3}  var {:.3}",
                        candidate.score,
                        candidate.final_mean,
                        candidate.activity,
                        candidate.variance
                    ));
                    if ui.button(format!("Apply Result {index}")).clicked() {
                        apply_result_index = Some(index);
                    }
                }
                if let Some(index) = apply_result_index {
                    self.params = self.explorer_results[index].params.clone();
                    self.ensure_beta_count();
                }
            }
        });

        ui.separator();
        ui.heading("Kernel Preview");
        ui.label(match self.params.kernel_mode {
            KernelMode::CenteredGaussian => {
                "Heatmap and radial average from kernel_size, num_peaks, and Gaussian widths."
            }
            KernelMode::GaussianRings => {
                "Heatmap and radial average from a Gaussian core with ring boosts."
            }
            KernelMode::LeniaBands => {
                "Heatmap and radial average from the official Lenia shell kernel and selected kernel core."
            }
        });
        let wide_layout = ui.available_width() >= 280.0;
        if wide_layout {
            ui.horizontal(|ui| {
                if let Some(texture) = self.kernel_texture.as_ref() {
                    let side = 128.0_f32.min(ui.available_width() * 0.45).max(110.0);
                    ui.add(egui::Image::new((texture.id(), egui::vec2(side, side))));
                }
                let plot_width = ui.available_width().max(110.0);
                self.draw_radial_kernel_plot(ui, egui::vec2(plot_width, 128.0));
            });
        } else {
            if let Some(texture) = self.kernel_texture.as_ref() {
                let side = ui.available_width().min(220.0).max(120.0);
                ui.add(egui::Image::new((texture.id(), egui::vec2(side, side))));
            }
            self.draw_radial_kernel_plot(ui, egui::vec2(ui.available_width().max(120.0), 128.0));
        }
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        let Some(texture) = self.texture.as_ref() else {
            ui.label("No simulation texture");
            return;
        };

        let texture_id = texture.id();
        let texture_size = texture.size_vec2();
        let available = ui.available_size();
        let scale = (available.x / texture_size.x)
            .min(available.y / texture_size.y)
            .max(0.1);
        let image_size = texture_size * scale;

        let response =
            ui.add(egui::Image::new((texture_id, image_size)).sense(Sense::click_and_drag()));
        let painting = response.dragged()
            || (response.hovered() && ui.input(|input| input.pointer.primary_down()))
            || response.clicked();

        if !painting {
            return;
        }

        let Some(pointer_pos) = response.interact_pointer_pos() else {
            return;
        };

        let rect = response.rect;
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }

        let x = ((pointer_pos.x - rect.left()) / rect.width()).clamp(0.0, 0.999_999_94);
        let y = ((pointer_pos.y - rect.top()) / rect.height()).clamp(0.0, 0.999_999_94);
        let col = (x * self.world.ncols() as f32) as usize;
        let row = (y * self.world.nrows() as f32) as usize;
        self.apply_tool(col, row);
    }
}

impl eframe::App for LeniaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.running {
            for _ in 0..self.steps_per_frame.max(1) {
                self.step_once();
            }
        }

        self.refresh_texture(ctx);
        self.refresh_kernel_texture(ctx);

        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.draw_controls(ui));
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Lenia");
            ui.separator();
            self.draw_canvas(ui);
        });

        ctx.request_repaint();
    }
}

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 900.0])
            .with_min_inner_size([900.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Lenia (egui)",
        options,
        Box::new(|cc| Ok(Box::new(LeniaApp::new(cc)))),
    )
}
