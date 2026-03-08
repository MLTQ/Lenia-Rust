# main.rs

## Purpose
Native executable entrypoint that launches the egui Lenia frontend.

## Components

### `main`
- **Does**: Delegates process startup to `lenia_3::app::run()`.
- **Interacts with**: `run` in `app.rs`.

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `cargo run` users | Starts GUI app and returns `eframe::Result<()>` | Changing return type or removing call into app module |
