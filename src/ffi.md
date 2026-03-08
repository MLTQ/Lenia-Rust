# ffi.rs

## Purpose
Provides the `run_lenia` C ABI function so external callers (Python ctypes frontend) can execute one simulation step.

## Components

### `run_lenia`
- **Does**: Validates pointers/dimensions, maps raw buffers to ndarray views, builds `LeniaParams`, and writes stepped output.
- **Interacts with**: `run_step` in `lenia.rs` and dynamic library consumers.
- **Rationale**: Keeps unsafe FFI boundary isolated from simulation math and UI code.

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `tester.py` | Signature and parameter order match ctypes definition | Reordering args / changing numeric types |
| cdylib export | Symbol name is exactly `run_lenia` | Renaming/removing `#[no_mangle]` |
| `lenia.rs` | `run_step` accepts owned input and returns output matrix | Return type/signature changes |
