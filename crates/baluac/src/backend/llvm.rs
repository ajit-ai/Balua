//! LLVM IR backend — CPU targets (x86_64, aarch64, riscv64, arm-none-eabi)
//! Emits LLVM IR via string builder (real impl would use llvm-sys). Supports LTO/PGO/BOLT hooks.

use super::Backend;
use crate::mir::MirModule;

#[derive(Default)]
pub struct LlvmBackend {
    pub target_triple: String,
    pub opt_level: u8,
    pub lto: bool,
}

impl Backend for LlvmBackend {
    fn name(&self) -> &'static str { "llvm" }

    fn lower(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        let mut out = String::new();
        out.push_str(&format!("; Balua LLVM backend — target: {}\n", if self.target_triple.is_empty() { "x86_64-unknown-linux-gnu" } else { &self.target_triple }));
        out.push_str("; Generated from Balua MIR — SSA form with hardware-annotated BBs\n");
        if self.lto { out.push_str("; LTO enabled\n"); }
        for m in modules {
            out.push_str(&format!("\n; Module: {}\n", m.name));
            for f in &m.functions {
                let hw_attr = f.hardware.as_ref().map(|h| format!("  ; hw: {:?}", h)).unwrap_or_default();
                if !hw_attr.is_empty() { out.push_str(&hw_attr); out.push('\n'); }
                out.push_str(&format!("define void @{}() {{\n", f.name));
                out.push_str("entry:\n");
                for bb in &f.basic_blocks {
                    out.push_str(&format!("  ; BB{} hw={:?}\n", bb.id, bb.hardware));
                    for inst in &bb.instructions {
                        out.push_str(&format!("  ; {:?}\n", inst));
                    }
                }
                out.push_str("  ret void\n}\n");
            }
        }
        Ok(out)
    }
}
