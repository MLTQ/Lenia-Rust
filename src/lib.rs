pub mod app;
mod ffi;
pub mod lenia;
pub mod species;

pub use ffi::run_lenia;
pub use lenia::{GrowthFuncType, KernelCoreType, KernelMode, LeniaParams};
