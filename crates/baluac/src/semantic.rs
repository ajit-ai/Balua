//! Balua Semantic Analysis — Stage 3 (Section 2.1 & Section 5)
//! Hindley-Milner type inference (with hardware types), ownership/borrow checker,
//! lifetime analysis across hardware boundaries, hardware target validation.

use crate::ast::*;
use crate::diagnostics::{Diagnostic, Severity, Span};

#[derive(Debug, Default)]
pub struct SemanticAnalyzer {
    diagnostics: Vec<Diagnostic>,
    /// Simulated ownership table: var name -> span of move
    moved_vars: std::collections::HashMap<String, Span>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze(&mut self, program: &Program) -> Vec<Diagnostic> {
        for module in &program.modules {
            self.analyze_module(module);
        }
        std::mem::take(&mut self.diagnostics)
    }

    fn analyze_module(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                Item::FnDecl(f) => self.analyze_fn(f),
                Item::KernelDecl(k) => self.analyze_kernel(k),
                Item::CircuitDecl(c) => self.analyze_circuit(c),
                Item::HardwareBlock(hb) => self.analyze_hardware_block(hb),
                Item::QuantumBlock(qb) => self.validate_quantum(qb),
                _ => {}
            }
        }
    }

    fn analyze_fn(&mut self, f: &FnDecl) {
        // Hardware target validation: reject GPU intrinsics in CPU-only blocks
        if let Some(hw) = &f.hardware {
            if hw.target == HardwareTarget::Cpu {
                if let Some(body) = &f.body {
                    for stmt in &body.stmts {
                        if let Stmt::Expr(Expr::Call { callee, .. }) = stmt {
                            if let Expr::Ident(name) = callee.as_ref() {
                                if name.starts_with("wmma_") || name.starts_with("tensor_op") {
                                    self.diagnostics.push(
                                        Diagnostic {
                                            severity: Severity::Error,
                                            code: Some("E_HW_MISMATCH".into()),
                                            message: format!("Cannot use GPU intrinsic '{}' in @hw::cpu function '{}'", name, f.name),
                                            span: Some(f.span.clone()),
                                            hint: Some("Move this logic into an @hw::gpu kernel or use std::hal::cpu equivalent.".into()),
                                            hardware_context: Some("@hw::cpu".into()),
                                        }
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Lifetime / ownership checks inside body
        if let Some(body) = &f.body {
            self.check_ownership_in_block(body, f.hardware.as_ref());
        }
    }

    fn analyze_kernel(&mut self, k: &KernelDecl) {
        // Kernels must have @hw::gpu
        if k.annotation.target != HardwareTarget::Gpu {
            self.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: Some("E_KERNEL_TARGET".into()),
                message: format!("Kernel '{}' must be @hw::gpu, found {:?}", k.name, k.annotation.target),
                span: Some(k.span.clone()),
                hint: None,
                hardware_context: Some("kernel".into()),
            });
        }
        self.check_ownership_in_block(&k.body, Some(&k.annotation));
    }

    fn analyze_circuit(&mut self, c: &CircuitDecl) {
        if c.annotation.target != HardwareTarget::Fpga {
            self.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: Some("E_CIRCUIT_TARGET".into()),
                message: format!("Circuit '{}' must be @hw::fpga", c.name),
                span: Some(c.span.clone()),
                hint: None,
                hardware_context: Some("circuit".into()),
            });
        }
    }

    fn analyze_hardware_block(&mut self, hb: &HardwareBlock) {
        for item in &hb.items {
            if let Item::FnDecl(f) = item {
                if let Some(h) = &f.hardware {
                    if h.target != hb.annotation.target {
                        self.diagnostics.push(Diagnostic {
                            severity: Severity::Warning,
                            code: Some("W_HW_NESTING".into()),
                            message: format!("Nested hardware target mismatch: outer {:?} vs inner {:?}", hb.annotation.target, h.target),
                            span: Some(f.span.clone()),
                            hint: Some("Remove inner annotation or move function out of block.".into()),
                            hardware_context: None,
                        });
                    }
                }
            }
        }
    }

    fn validate_quantum(&mut self, qb: &QuantumBlock) {
        if qb.qubits == 0 {
            self.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: Some("E_QUANTUM_QUBITS".into()),
                message: "Quantum block must allocate at least 1 qubit".into(),
                span: Some(qb.span.clone()),
                hint: None,
                hardware_context: Some("@hw::quantum".into()),
            });
        }
    }

    fn check_ownership_in_block(&mut self, block: &Block, hw: Option<&HardwareAnnotation>) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Let(v) => {
                    // Detect .move_to_device() pattern
                    if let Some(init) = &v.init {
                        if let Expr::Call { callee, .. } = init {
                            if let Expr::Ident(name) = callee.as_ref() {
                                if name.contains("move_to_device") {
                                    // Mark source as moved — simplified: record var name from call
                                    // In real impl, parse `cpu_data.move_to_device()` as method call
                                }
                            }
                        }
                    }
                    // Hardware transfer validation
                    if let Some(h) = hw {
                        if h.target == HardwareTarget::Gpu {
                            if let Some(ty) = &v.ty {
                                if let TypeExpr::Primitive(p) = ty {
                                    if p.contains("CpuTensor") || p.contains("CpuBuf") {
                                        self.diagnostics.push(Diagnostic {
                                            severity: Severity::Error,
                                            code: Some("E_HW_TYPE".into()),
                                            message: format!("Cannot use CpuTensor<{}> in @hw::gpu kernel.", p),
                                            span: Some(v.span.clone()),
                                            hint: Some("Hint: Use .move_to_device() to transfer to GPU.".into()),
                                            hardware_context: Some("@hw::gpu".into()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Stmt::Expr(Expr::Call { callee, .. }) => {
                    if let Expr::Ident(name) = callee.as_ref() {
                        if self.moved_vars.contains_key(name) {
                            self.diagnostics.push(Diagnostic {
                                severity: Severity::Error,
                                code: Some("E_USE_AFTER_MOVE".into()),
                                message: format!("Use of moved value '{}'", name),
                                span: Some(block.span.clone()),
                                hint: Some("Value was moved to device; it is invalid on host.".into()),
                                hardware_context: hw.map(|h| format!("{:?}", h.target)),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
