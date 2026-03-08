pub mod app;
mod ffi;
pub mod lenia;

pub use ffi::run_lenia;
pub use lenia::{GrowthFuncType, LeniaParams};
