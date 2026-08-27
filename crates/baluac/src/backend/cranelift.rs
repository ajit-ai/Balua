//! Cranelift backend — lightweight CPU alternative to LLVM (4GB i7, Windows 11)
//! Mirrors llvm.rs API. Emits Cranelift IR (.clif) for debug builds. Production still LLVM thin LTO.

use super::Backend;
use crate::mir::{MirModule, Instruction, Terminator};

#[derive(Default)]
pub struct CraneliftBackend {
    pub target_triple: String,
    pub opt_level: u8,
}

impl Backend for CraneliftBackend {
    fn name(&self) -> &'static str { "cranelift" }
    fn lower(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        let triple = if self.target_triple.is_empty() { "x86_64-pc-windows-msvc" } else { &self.target_triple };
        let mut out = format!("; Balua Cranelift backend — target: {} (opt={}) — 4GB host fast debug\n", triple, self.opt_level);
        out.push_str("; CLIF: Cranelift IR — no LTO/BOLT needed, incremental\n");
        for m in modules {
            out.push_str(&format!("\n; Module: {}\n", m.name));
            for f in &m.functions {
                out.push_str(&format!("function @{}() system_v {{\n", f.name));
                out.push_str("block0:\n");
                for bb in &f.basic_blocks {
                    for inst in &bb.instructions {
                        out.push_str(&format!("  ; {}\n", clif_inst(inst)));
                    }
                    match &bb.terminator {
                        Terminator::Return(Some(v)) => out.push_str(&format!("  return {}\n", v)),
                        Terminator::Return(None) => out.push_str("  return 0\n"),
                        Terminator::Jump(t) => out.push_str(&format!("  jump block{}\n", t)),
                        Terminator::Branch { cond, then_bb, else_bb } => out.push_str(&format!("  brif {}, block{}, block{}\n", cond, then_bb, else_bb)),
                        Terminator::Unreachable => out.push_str("  trap\n"),
                    }
                }
                out.push_str("}\n");
            }
        }
        Ok(out)
    }
}

fn clif_inst(inst: &Instruction) -> String {
    match inst {
        Instruction::BinOp { dest, op, lhs, rhs } => format!("{} = {} {}, {}", dest, op, lhs, rhs),
        _ => format!("{:?}", inst),
    }
}
