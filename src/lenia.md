# lenia.rs

## Purpose
Contains Lenia simulation math, parameter types, single-step evolution, and brush/blob utilities shared by both frontends.

## Components

### `GrowthFuncType`
- **Does**: Enumerates growth function modes (Polynomial, Exponential, Step).
- **Interacts with**: UI selectors in `app.rs` and FFI arguments in `ffi.rs`.

### `KernelMode`
- **Does**: Selects between the legacy centered-Gaussian kernel and a separate Gaussian-rings kernel path.
- **Interacts with**: UI controls in `app.rs`, `generate_kernel`, and default FFI behavior.

### `LeniaParams`
- **Does**: Stores tunable simulation parameters, kernel mode, presets, and beta normalization rules per mode.
- **Interacts with**: `run_step`, egui controls, and `run_lenia`.

### `run_step`
- **Does**: Runs a full Lenia update step: kernel generation, convolution, growth mapping, and clamped integration.
- **Interacts with**: Called by `LeniaApp::step_once` and `ffi::run_lenia`.

### `generate_kernel`
- **Does**: Dispatches to the active kernel generator based on `KernelMode`.
- **Interacts with**: `run_step` and kernel preview rendering in `app.rs`.

### `generate_centered_gaussian_kernel`
- **Does**: Builds the original stable kernel by averaging multiple center-aligned Gaussian components.
- **Interacts with**: Used directly for `KernelMode::CenteredGaussian` and as the mass reference for ring mode.

### `generate_gaussian_rings_kernel`
- **Does**: Builds a Gaussian-core kernel with additive ring lobes and rescales it to match the legacy kernel mass.
- **Interacts with**: Used for `KernelMode::GaussianRings` to keep convolution magnitudes in a similar range.

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
| `app.rs` | `generate_kernel` returns a non-empty kernel for the selected mode | Removing/changing mode dispatch or kernel mass scaling |
| `ffi.rs` | `GrowthFuncType` is `#[repr(C)]` and stable | Reordering/removing enum variants |
| `ffi.rs` | FFI calls always use `KernelMode::CenteredGaussian` unless the ABI is extended | Changing default FFI kernel mode |
| UI controls | `LeniaParams::normalized_betas` pads/truncates safely | Panics or changing beta normalization behavior |

## Notes
- Convolution uses toroidal wraparound to match prior behavior.
- `kernel_size` is forced odd internally so the kernel remains centered.
- In `CenteredGaussian`, `betas` are Gaussian widths.
- In `GaussianRings`, `betas[0]` is the core width and later entries are ring boost amplitudes.
