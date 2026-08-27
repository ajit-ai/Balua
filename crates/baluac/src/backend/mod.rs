//! Balua backends — Section 2.2
//! Backend selection driven by @hw:: annotations. Mixed-hardware programs
//! compile each annotated region to its backend; baluald stitches objects.

pub mod hls;
pub mod llvm;
pub mod mlir_dialect;
pub mod openqasm;
pub mod ptx;
pub mod spirv;

use crate::mir::MirModule;

pub trait Backend {
    fn name(&self) -> &'static str;
    fn lower(&self, modules: &[MirModule]) -> anyhow::Result<String>;
}

pub fn select_backend(hw: &str) -> Box<dyn Backend> {
    match hw {
        "llvm" | "cpu" => Box::new(llvm::LlvmBackend::default()),
        "ptx" | "cuda" => Box::new(ptx::PtxBackend::default()),
        "spirv" | "vulkan" => Box::new(spirv::SpirvBackend::default()),
        "hls" | "fpga" => Box::new(hls::HlsBackend::default()),
        "openqasm" | "quantum" => Box::new(openqasm::QasmBackend::default()),
        "mlir" => Box::new(mlir_dialect::MlirBackend::default()),
        _ => Box::new(llvm::LlvmBackend::default()),
    }
}
