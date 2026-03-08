use crate::lenia::{
    apply_circular_brush, random_world, run_step, stamp_gaussian_blob, GrowthFuncType, LeniaParams,
};
use eframe::egui::{self, ColorImage, Sense, TextureHandle, TextureOptions};
use ndarray::Array2;
use rand::Rng;

const WORLD_SIZE: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaintTool {
    DrawLife,
    Erase,
    PlaceFood,
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
    running: bool,
    steps_per_frame: usize,
    paint_tool: PaintTool,
    brush_radius: usize,
    brush_strength: f64,
    food: FoodSettings,
    frame_counter: usize,
    texture: Option<TextureHandle>,
}

impl LeniaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            world: random_world(WORLD_SIZE, WORLD_SIZE),
            params: LeniaParams::default(),
            running: true,
            steps_per_frame: 1,
            paint_tool: PaintTool::DrawLife,
            brush_radius: 4,
            brush_strength: 0.2,
            food: FoodSettings::default(),
            frame_counter: 0,
            texture: None,
        };
        app.ensure_beta_count();
        app.regenerate_food_sources();
        app.apply_food_sources();
        app
    }

    fn ensure_beta_count(&mut self) {
        self.params.betas = self.params.normalized_betas();
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
        let grayscale: Vec<u8> = self
            .world
            .iter()
            .map(|value| (value.clamp(0.0, 1.0) * 255.0) as u8)
            .collect();
        ColorImage::from_gray([self.world.ncols(), self.world.nrows()], &grayscale)
    }

    fn refresh_texture(&mut self, ctx: &egui::Context) {
        let image = self.world_to_image();
        if let Some(texture) = &mut self.texture {
            texture.set(image, TextureOptions::NEAREST);
        } else {
            self.texture = Some(ctx.load_texture("lenia-world", image, TextureOptions::NEAREST));
        }
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
            }
            if ui.button("Clear").clicked() {
                self.world.fill(0.0);
            }
        });
        ui.add(egui::Slider::new(&mut self.steps_per_frame, 1..=8).text("steps/frame"));
        ui.label(format!(
            "Mean population: {:.3}",
            self.world.sum() / (self.world.len() as f64)
        ));

        ui.separator();
        ui.collapsing("Lenia Parameters", |ui| {
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
                ui.add(
                    egui::DragValue::new(beta)
                        .speed(0.05)
                        .range(0.01..=25.0)
                        .prefix(format!("beta[{index}] ")),
                );
            }

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

        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| self.draw_controls(ui));

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
