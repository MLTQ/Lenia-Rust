# tester.py

## Purpose
Legacy Python/Pygame frontend that loads the Rust cdylib through ctypes, runs simulation steps, and optionally injects periodic food blobs.

## Components

### `GrowthFuncType`
- **Does**: Mirrors Rust growth enum values for FFI calls.
- **Interacts with**: `run_lenia` argument `growth_func_type`.

### `generate_gaussian_blob`
- **Does**: Produces food blob values used for periodic injection.
- **Interacts with**: Initial world seeding and timed food refresh.

### Main loop
- **Does**: Advances simulation each frame, applies optional food, and renders upscaled grayscale output with pygame.
- **Interacts with**: Rust `run_lenia` export in generated `liblenia_3.dylib`.

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| Python runner | `target/release/liblenia_3.dylib` is present and ABI-compatible | Library path or ABI change |
| Users | Parameters near top of file control behavior | Removing global parameter block |
