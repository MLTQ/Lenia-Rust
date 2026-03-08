# app.rs

## Purpose
Implements a native `eframe/egui` frontend for Lenia with full parameter tuning, simulation controls, and interactive drawing/food placement.

## Components

### `LeniaApp`
- **Does**: Owns world state, current simulation params, tool state, food settings, and display texture.
- **Interacts with**: Utility functions in `lenia.rs` and `eframe::App` runtime.

### `LeniaApp::draw_controls`
- **Does**: Renders play/pause, step, randomize/clear, all Lenia parameter controls, food settings, and drawing tool controls.
- **Interacts with**: Mutates `LeniaApp` fields and calls helper methods (`apply_food_sources`, `regenerate_food_sources`).

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

## Notes
- Texture updates use nearest filtering so each automaton cell remains crisp.
- The app continuously repaints while running to keep simulation and drawing responsive.
