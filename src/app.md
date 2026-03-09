# app.rs

## Purpose
Implements a native `eframe/egui` frontend for Lenia with full parameter tuning, simulation controls, resizable fields, alternate color scales, interactive drawing/food placement, and a live kernel preview panel with both a heatmap and radial plot.

## Components

### `LeniaApp`
- **Does**: Owns world state, current simulation params, field sizing state, color scale state, tool state, food settings, and display textures.
- **Interacts with**: Utility functions in `lenia.rs` and `eframe::App` runtime.

### `LeniaApp::draw_controls`
- **Does**: Renders play/pause, step, randomize/clear, field size controls, color scale controls, all Lenia parameter controls, food settings, and drawing tool controls.
- **Interacts with**: Mutates `LeniaApp` fields and calls helper methods (`apply_food_sources`, `regenerate_food_sources`).
- **Rationale**: Intended to live inside a vertical scroll area so lower controls and kernel previews remain reachable on shorter windows.

### `LeniaApp::apply_centered_gaussian_preset` and `LeniaApp::apply_gaussian_rings_preset`
- **Does**: Loads stable parameter bundles for the legacy Gaussian kernel and the experimental ringed kernel.
- **Interacts with**: `LeniaParams` presets in `lenia.rs` and the preset buttons in the controls pane.

### `LeniaApp::resize_world`
- **Does**: Reallocates the field to a requested size while preserving the centered overlapping region of the old state.
- **Interacts with**: Field size controls, brush mapping, and food source regeneration.

### `LeniaApp::kernel_to_image` and `LeniaApp::refresh_kernel_texture`
- **Does**: Builds a normalized kernel heatmap image from current parameters and uploads/updates an egui texture.
- **Interacts with**: `generate_kernel` in `lenia.rs` and the bottom settings pane rendering.

### `ColorScale`, `sample_gradient`, and `values_to_color_image`
- **Does**: Maps scalar field values into selectable RGB palettes shared by the main simulation view and the kernel preview.
- **Interacts with**: `world_to_image`, `kernel_to_image`, and the color scale dropdown.

### `LeniaApp::kernel_radial_profile` and `LeniaApp::draw_radial_kernel_plot`
- **Does**: Computes a radius-averaged 1D kernel profile and renders it as a lightweight painter-based chart.
- **Interacts with**: `generate_kernel` in `lenia.rs` and the kernel preview section in `draw_controls`.

### `LeniaApp::draw_canvas`
- **Does**: Draws world texture and maps pointer coordinates to grid coordinates for brush interactions.
- **Interacts with**: `apply_tool`, brush settings, and world array dimensions.

### `LeniaApp::step_once`
- **Does**: Runs one Lenia simulation step and applies periodic food injection when configured.
- **Interacts with**: `run_step` and food settings.

### `run`
- **Does**: Starts the native egui application with an initial viewport size.
- **Interacts with**: Called by `src/main.rs`.

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `main.rs` | `run()` returns `eframe::Result<()>` | Signature change |
| User interaction | Drag/click painting updates simulation grid immediately | Removing pointer-to-grid mapping |
| Existing workflow | Food refresh supports fixed and randomized source placement | Removing periodic food controls |
| Parameter tuning UX | Bottom pane shows current kernel heatmap and radial profile for the selected kernel mode | Removing or desynchronizing preview refresh |
| Runtime sizing | Applying a new field size preserves centered content and updates pointer mapping to new dimensions | Breaking resize semantics or leaving stale dimensions |

## Notes
- Texture updates use nearest filtering so each automaton cell remains crisp.
- The app continuously repaints while running to keep simulation and drawing responsive.
- Kernel preview texture and radial plot are refreshed each frame to stay in sync with slider changes.
- The settings panel is scrollable so the kernel preview remains accessible even when the window is short.
- Kernel mode changes do not affect the Python FFI path; they are only exposed through the native app.
- Color scales affect both the simulation field and the kernel preview so the palette stays visually consistent.
