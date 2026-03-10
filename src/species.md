# species.rs

## Purpose
Owns the curated published-species library for the native app. It decodes the upstream Lenia archive format, maps official `R/T/b/m/s/kn/gn` settings into local runtime parameters, and builds centered worlds ready to load into the simulator.

## Components

### `SpeciesPreset`
- **Does**: Stores one embedded species entry from the upstream Lenia archive, including official parameters and compact cell data.
- **Interacts with**: `LeniaApp` in `app.rs` and `LeniaParams::from_official_lenia` in `lenia.rs`.

### `LoadedSpecies`
- **Does**: Bundles the decoded world and translated runtime parameters for immediate use by the app.
- **Interacts with**: `LeniaApp::apply_selected_species`.

### `curated_species`
- **Does**: Returns the embedded curated catalog.
- **Interacts with**: Species dropdown in `app.rs`.

### `SpeciesPreset::load`
- **Does**: Decodes the archived pattern, parses shell weights, builds official Lenia params, and centers the result inside a recommended world size.
- **Interacts with**: `decode_rle_2d`, `parse_shell_weights`, and `LeniaParams::from_official_lenia`.

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `app.rs` | `curated_species()` returns stable embedded presets that can be loaded without I/O | Changing preset storage or load semantics |
| `app.rs` | `SpeciesPreset::load()` produces a non-empty centered world and matching params | Returning empty worlds or mismatched params |
| `lenia.rs` | Official `kn`/`gn` mappings are already represented by `KernelCoreType`/`GrowthFuncType` | Diverging enum semantics |

## Notes
- This is intentionally a curated subset, not a full importer for the entire upstream archive.
- The compact cell decoder implements the upstream 2D RLE variant used in `animals.json`.
- Recommended world size is derived from the archived pattern extents plus kernel radius padding so presets have room to evolve.
