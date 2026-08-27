//! OpenQASM 3.0 / Q# backend — Quantum targets

use super::Backend;
use crate::mir::MirModule;

#[derive(Default)]
pub struct QasmBackend { pub qubits: usize, pub emit_qsharp: bool }

impl Backend for QasmBackend {
    fn name(&self) -> &'static str { "openqasm" }
    fn lower(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        let mut out = String::from("OPENQASM 3.0;\ninclude \"stdgates.inc\";\n\n// Balua @hw::quantum — lowered from MIR\n");
        for m in modules {
            for f in &m.functions {
                if !matches!(f.hardware, Some(crate::ast::HardwareTarget::Quantum)) { continue; }
                let q = if self.qubits == 0 { 5 } else { self.qubits };
                out.push_str(&format!("qubit[{}] q;\nbit[{}] c;\n", q, q));
                out.push_str(&format!("// Circuit: {}\n", f.name));
                out.push_str("h q; // Hadamard on all qubits — placeholder\n");
                out.push_str("barrier q;\n");
                out.push_str("c = measure q;\n\n");
                if self.emit_qsharp {
                    out.push_str("// Q# emission (Azure Quantum):\n");
                    out.push_str(&format!("// operation {}() : Result[] {{ use q = Qubit[{}]; H(q[0]); return [M(q[0])]; }}\n\n", f.name, q));
                }
            }
        }
        Ok(out)
    }
}
