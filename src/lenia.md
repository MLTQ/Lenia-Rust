# lenia.rs

## Purpose
Contains Lenia simulation math, parameter types, single-step evolution, and brush/blob utilities shared by both frontends.

## Components

### `GrowthFuncType`
- **Does**: Enumerates growth function modes (Polynomial, Exponential, Step).
- **Interacts with**: UI selectors in `app.rs` and FFI arguments in `ffi.rs`.

### `LeniaParams`
- **Does**: Stores tunable simulation parameters and normalizes beta vectors for peak count consistency.
- **Interacts with**: `run_step`, egui controls, and `run_lenia`.

### `run_step`
- **Does**: Runs a full Lenia update step: kernel generation, convolution, growth mapping, and clamped integration.
- **Interacts with**: Called by `LeniaApp::step_once` and `ffi::run_lenia`.

### `apply_circular_brush`
- **Does**: Adds/removes life around a brush center with circular falloff mask.
- **Interacts with**: Drawing tools in `app.rs`.

### `stamp_gaussian_blob`
- **Does**: Adds food-style Gaussian rings/blobs to the world state.
- **Interacts with**: Food refresh logic and “Place Food” draw mode in `app.rs`.

### `random_world`
- **Does**: Generates randomized initial population grids.
- **Interacts with**: App startup and “Randomize” control.

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `app.rs` | `run_step` returns a same-size clamped world | Shape/value range contract changes |
| `ffi.rs` | `GrowthFuncType` is `#[repr(C)]` and stable | Reordering/removing enum variants |
| UI controls | `LeniaParams::normalized_betas` pads/truncates safely | Panics or changing beta normalization behavior |

## Notes
- Convolution uses toroidal wraparound to match prior behavior.
- `kernel_size` is forced odd internally so the kernel remains centered.
