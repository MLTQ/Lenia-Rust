# lib.rs

## Purpose
Library entrypoint that exposes Lenia simulation primitives, the native egui app module, and the C ABI bridge used by the Python frontend.

## Components

### `pub mod app`
- **Does**: Exposes the native `eframe/egui` frontend.
- **Interacts with**: `LeniaApp` and `run()` in `app.rs`.

### `mod ffi` and `pub use ffi::run_lenia`
- **Does**: Keeps the exported C symbol available from the library.
- **Interacts with**: `run_step` and `LeniaParams` in `lenia.rs`.

### `pub mod lenia`
- **Does**: Hosts all simulation math and grid-edit helpers.
- **Interacts with**: Both `app.rs` and `ffi.rs`.

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `src/main.rs` | `app::run()` is accessible through crate exports | Renaming/removing `app` module |
| Python ctypes loader | `run_lenia` symbol is exported from cdylib | Symbol name/signature changes |
| Internal Rust code | `GrowthFuncType`, `LeniaParams` are re-exported | Removing/retyping these re-exports |
