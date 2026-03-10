# lenia.rs

## Purpose
Contains Lenia simulation math, parameter types, single-step evolution, official Lenia band-kernel support, and brush/blob utilities shared by both frontends.

## Components

### `GrowthFuncType`
- **Does**: Enumerates growth function modes (Polynomial, Exponential, Step) and maps official Lenia `gn` indices.
- **Interacts with**: UI selectors in `app.rs`, species loading in `species.rs`, and FFI arguments in `ffi.rs`.

### `KernelCoreType`
- **Does**: Enumerates official Lenia kernel-core shapes (Polynomial, Exponential, Step, Staircase).
- **Interacts with**: `generate_lenia_bands_kernel`, species loading in `species.rs`, and UI selectors in `app.rs`.

### `KernelMode`
- **Does**: Selects between the legacy centered-Gaussian kernel, a separate Gaussian-rings kernel path, and an official segmented Lenia band kernel.
- **Interacts with**: UI controls in `app.rs`, `generate_kernel`, species loading, and default FFI behavior.

### `LeniaParams`
- **Does**: Stores tunable simulation parameters, kernel/core modes, presets, official-species conversion, and beta normalization rules per mode.
- **Interacts with**: `run_step`, egui controls, species loading in `species.rs`, and `run_lenia`.

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

### `generate_lenia_bands_kernel`
- **Does**: Builds the official Lenia shell kernel by applying shell weights over normalized radial bands and normalizing the result.
- **Interacts with**: Used for `KernelMode::LeniaBands`, species presets, and the kernel preview in `app.rs`.

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
| `app.rs` | `generate_kernel` returns a non-empty kernel for the selected mode | Removing/changing mode dispatch or kernel normalization |
| `ffi.rs` | `GrowthFuncType` is `#[repr(C)]` and stable | Reordering/removing enum variants |
| `ffi.rs` | FFI calls always use `KernelMode::CenteredGaussian` unless the ABI is extended | Changing default FFI kernel mode |
| `species.rs` | `from_official_lenia` maps archived `R/T/b/m/s/kn/gn` fields into working runtime params | Breaking official index mapping or kernel-size semantics |
| UI controls | `LeniaParams::normalized_betas` pads/truncates safely | Panics or changing beta normalization behavior |

## Notes
- Convolution uses toroidal wraparound to match prior behavior.
- `kernel_size` is forced odd internally so the kernel remains centered.
- In `CenteredGaussian`, `betas` are Gaussian widths.
- In `GaussianRings`, `betas[0]` is the core width and later entries are ring boost amplitudes.
- In `LeniaBands`, `kernel_size` is interpreted as `2R+1`, `betas` are shell weights, and `kernel_core_type` selects the within-band profile.
