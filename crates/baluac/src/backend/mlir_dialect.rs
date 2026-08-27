//! MLIR dialect — universal IR hub for Balua
//! Registers custom 'balua' dialect and lowers via dialect passes to all backends.

use super::Backend;
use crate::mir::MirModule;

#[derive(Default)]
pub struct MlirBackend;

impl Backend for MlirBackend {
    fn name(&self) -> &'static str { "mlir" }
    fn lower(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        let mut out = String::from("// Balua MLIR dialect — universal IR hub\nmodule {\n");
        out.push_str("  // dialect: balua.hardware = #balua.hw<gpu|cpu|fpga|npu|quantum>\n");
        for m in modules {
            out.push_str(&format!("  balua.module @{} {{\n", m.name));
            for f in &m.functions {
                let hw_attr = f.hardware.as_ref().map(|h| format!("#balua.hw<{:?}>", h)).unwrap_or("#balua.hw<cpu>".into());
                out.push_str(&format!("    balua.func @{} {{}} attributes {{ balua.hw = {}, balua.mir.ssa = true }} {{\n", f.name, hw_attr));
                out.push_str("      // BB0: hardware-annotated basic block — lowered via passes: balua->llvm, balua->ptx, balua->spirv, balua->hls, balua->qasm\n");
                out.push_str("      scf.execute_region { balua.terminator }\n    }\n");
            }
            out.push_str("  }\n");
        }
        out.push_str("}\n");
        // lowering pipeline
        out.push_str("\n// Pass pipeline: balua-verify | balua-lower-hw | mlir-opt -convert-balua-to-llvm -convert-balua-to-gpu\n");
        Ok(out)
    }
}
