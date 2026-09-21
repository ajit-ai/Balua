//! Balua Semantic Analysis — Stage 3 (Section 2.1 & Section 5)
//! Hindley-Milner type inference (with hardware types), ownership/borrow checker,
//! lifetime analysis across hardware boundaries, hardware target validation.
//!
//! Phase 1: Wires InferenceEngine (types/inference.rs) and BorrowChecker
//! (types/borrow_checker.rs) into the semantic pass. Produces a TypeTable
//! (expression span → resolved Ty) for downstream MIR/codegen consumers.

use crate::ast::*;
use crate::diagnostics::{Diagnostic, Severity, Span};
use crate::types::borrow_checker::{BorrowChecker, BorrowKind};
use crate::types::inference::{InferenceEngine, Ty};
use std::collections::HashMap;

/// Side-table mapping expression spans to resolved types.
/// Populated by SemanticAnalyzer::analyze; consumed by MIR/codegen (Phase 2+).
pub type TypeTable = HashMap<Span, Ty>;

/// Scoped type environment: maps variable names to their inferred Ty.
struct TypeEnv {
    scopes: Vec<HashMap<String, Ty>>,
}

impl TypeEnv {
    fn new() -> Self {
        Self { scopes: vec![HashMap::new()] }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert(&mut self, name: &str, ty: Ty) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<Ty> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }
}

pub struct SemanticAnalyzer {
    diagnostics: Vec<Diagnostic>,
    engine: InferenceEngine,
    borrow_checker: BorrowChecker,
    type_env: TypeEnv,
    type_table: TypeTable,
    /// Current hardware context (set when analyzing inside an @hw-annotated function)
    hw_context: Option<HardwareAnnotation>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            engine: InferenceEngine::new(),
            borrow_checker: BorrowChecker::new(),
            type_env: TypeEnv::new(),
            type_table: HashMap::new(),
            hw_context: None,
        }
    }

    /// Run semantic analysis. Returns diagnostics + a TypeTable for downstream consumers.
    pub fn analyze(&mut self, program: &Program) -> (Vec<Diagnostic>, TypeTable) {
        for module in &program.modules {
            self.analyze_module(module);
        }
        self.diagnostics.append(&mut self.borrow_checker.take_diagnostics());
        let diags = std::mem::take(&mut self.diagnostics);
        let table = std::mem::take(&mut self.type_table);
        (diags, table)
    }

    // ── Module / item dispatch ──────────────────────────────────────────

    fn analyze_module(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                Item::FnDecl(f) => self.analyze_fn(f),
                Item::KernelDecl(k) => self.analyze_kernel(k),
                Item::CircuitDecl(c) => self.analyze_circuit(c),
                Item::HardwareBlock(hb) => self.analyze_hardware_block(hb),
                Item::QuantumBlock(qb) => self.validate_quantum(qb),
                Item::ConstDecl(c) => self.analyze_const(c),
                _ => {}
            }
        }
    }

    // ── Constants ───────────────────────────────────────────────────────

    fn analyze_const(&mut self, c: &ConstDecl) {
        if c.visibility == Visibility::Priv {
            self.diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                code: Some("W_CONST_PRIV".into()),
                message: format!("Private const '{}' — consider pub for cross-module use", c.name),
                span: Some(c.span.clone()),
                hint: Some("Use pub const if referenced from another module.".into()),
                hardware_context: None,
            });
        }
        let val_ty = self.analyze_expr(&c.value);
        if let Some(decl_ty) = &c.ty {
            let expected = self.type_expr_to_ty(decl_ty);
            self.unify_or_error(&expected, &val_ty, &c.span);
        }
    }

    // ── Functions ───────────────────────────────────────────────────────

    fn analyze_fn(&mut self, f: &FnDecl) {
        for attr in &f.attrs {
            let name = attr.trim_start_matches("#[").trim_end_matches(']').split('(').next().unwrap_or("").trim();
            if name != "max_stack" && name != "wcet_cycles" {
                self.diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    code: Some("W_UNKNOWN_ATTR".into()),
                    message: format!("Unknown attribute '{}' on function '{}' — ignored", attr, f.name),
                    span: Some(f.span.clone()),
                    hint: Some("Supported: #[max_stack(N)], #[wcet_cycles(N)].".into()),
                    hardware_context: None,
                });
            }
        }
        if let Some(hw) = &f.hardware {
            if hw.target == HardwareTarget::Cpu {
                if let Some(body) = &f.body {
                    for stmt in &body.stmts {
                        if let Stmt::Expr(Expr::Call { callee, .. }) = stmt {
                            if let Expr::Ident(name) = callee.as_ref() {
                                if name.starts_with("wmma_") || name.starts_with("tensor_op") {
                                    self.diagnostics.push(Diagnostic {
                                        severity: Severity::Error,
                                        code: Some("E_HW_MISMATCH".into()),
                                        message: format!("Cannot use GPU intrinsic '{}' in @hw::cpu function '{}'", name, f.name),
                                        span: Some(f.span.clone()),
                                        hint: Some("Move this logic into an @hw::gpu kernel or use std::hal::cpu equivalent.".into()),
                                        hardware_context: Some("@hw::cpu".into()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        match f.safety {
            SafetyTier::Unsafe => {
                if f.body.is_none() {
                    self.diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        code: Some("E_SAFE_NO_BODY".into()),
                        message: format!("Unsafe function '{}' must have a body", f.name),
                        span: Some(f.span.clone()),
                        hint: Some("Add an unsafe { } block or mark as trusted.".into()),
                        hardware_context: None,
                    });
                }
            }
            SafetyTier::Trusted => {
                self.diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    code: Some("W_TRUSTED_FUNCTION".into()),
                    message: format!("Trusted function '{}' bypasses safety checks — audit required", f.name),
                    span: Some(f.span.clone()),
                    hint: Some("Ensure trusted function is reviewed by security team.".to_string()),
                    hardware_context: None,
                });
            }
            SafetyTier::Safe => {}
        }

        if let Some(body) = &f.body {
            self.type_env.push_scope();
            for p in &f.params {
                let param_ty = self.type_expr_to_ty(&p.ty);
                self.type_env.insert(&p.name, param_ty);
            }
            let prev_hw = self.hw_context.clone();
            self.hw_context = f.hardware.clone();
            let hw_ref = self.hw_context.clone();
            let fn_ty = self.analyze_block(body, hw_ref.as_ref());
            self.hw_context = prev_hw;
            if let Some(ret) = &f.ret_ty {
                let expected = self.type_expr_to_ty(ret);
                self.unify_or_error(&expected, &fn_ty, &f.span);
            }
            self.type_env.pop_scope();
        }
    }

    // ── Hardware-specific items ─────────────────────────────────────────

    fn analyze_kernel(&mut self, k: &KernelDecl) {
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
        self.analyze_block(&k.body, Some(&k.annotation));
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

    // ── Type inference (new in Phase 1) ─────────────────────────────────

    /// Convert an AST TypeExpr into a Ty for the inference engine.
    fn type_expr_to_ty(&mut self, te: &TypeExpr) -> Ty {
        match te {
            TypeExpr::Primitive(s) => match s.as_str() {
                "i8" => Ty::I8, "i16" => Ty::I16, "i32" => Ty::I32, "i64" => Ty::I64,
                "i128" => Ty::I128,
                "u8" => Ty::U8, "u16" => Ty::U16, "u32" => Ty::U32, "u64" => Ty::U64,
                "u128" => Ty::U128,
                "f16" => Ty::F16, "f32" => Ty::F32, "f64" => Ty::F64, "f128" => Ty::F128,
                "bool" => Ty::Bool, "char" => Ty::Char, "qubit" => Ty::Qubit,
                "str" => Ty::App { name: "str".into(), args: vec![] },
                other => Ty::Generic(other.into()),
            },
            TypeExpr::Reference { is_mut, inner, .. } => Ty::Ref { is_mut: *is_mut, inner: Box::new(self.type_expr_to_ty(inner)) },
            TypeExpr::Pointer { is_mut, inner } => Ty::Ptr { is_mut: *is_mut, inner: Box::new(self.type_expr_to_ty(inner)) },
            TypeExpr::Slice(inner) => Ty::App { name: "Slice".into(), args: vec![self.type_expr_to_ty(inner)] },
            TypeExpr::Array { inner, size } => Ty::Tensor { inner: Box::new(self.type_expr_to_ty(inner)), shape: vec![*size] },
            TypeExpr::Generic { name, args } => Ty::App { name: name.clone(), args: args.iter().map(|a| self.type_expr_to_ty(a)).collect() },
            TypeExpr::Chan(inner) => Ty::App { name: "Chan".into(), args: vec![self.type_expr_to_ty(inner)] },
            TypeExpr::Simd { inner, .. } => self.type_expr_to_ty(inner),
            TypeExpr::Tensor { ty, shape, .. } => Ty::Tensor { inner: Box::new(self.type_expr_to_ty(ty)), shape: shape.clone() },
            TypeExpr::Qubit(_) => Ty::Qubit,
            TypeExpr::Stream(inner) => Ty::App { name: "Stream".into(), args: vec![self.type_expr_to_ty(inner)] },
            TypeExpr::Hardware { base, .. } => Ty::Generic(base.clone()),
            TypeExpr::Function { params, ret } => {
                let param_tys: Vec<Ty> = params.iter().map(|p| self.type_expr_to_ty(p)).collect();
                let ret_ty = self.type_expr_to_ty(ret);
                let mut args = param_tys;
                args.push(ret_ty);
                Ty::App { name: "Fn".into(), args }
            }
            TypeExpr::Inferred => self.engine.fresh_var(),
        }
    }

    /// Attempt to unify two types; emit E_TYPE_MISMATCH on failure.
    fn unify_or_error(&mut self, expected: &Ty, actual: &Ty, span: &Span) {
        if let Err(msg) = self.engine.unify(expected, actual) {
            self.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: Some("E_TYPE_MISMATCH".into()),
                message: msg,
                span: Some(span.clone()),
                hint: None,
                hardware_context: None,
            });
        }
    }

    // ── Expression analysis (new in Phase 1) ────────────────────────────

    /// Analyze an expression, infer its type, and record it in the TypeTable.
    fn analyze_expr(&mut self, expr: &Expr) -> Ty {
        let ty = match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_, suffix) => match suffix.as_str() {
                    "i8" => Ty::I8, "i16" => Ty::I16, "i64" => Ty::I64, "i128" => Ty::I128,
                    "u8" => Ty::U8, "u16" => Ty::U16, "u32" => Ty::U32, "u64" => Ty::U64,
                    "u128" => Ty::U128,
                    _ => Ty::I32,
                },
                Literal::Float(_, suffix) => match suffix.as_str() {
                    "f16" => Ty::F16, "f64" => Ty::F64, "f128" => Ty::F128,
                    _ => Ty::F32,
                },
                Literal::Bool(_) => Ty::Bool,
                Literal::Str(_) => Ty::App { name: "str".into(), args: vec![] },
            },

            Expr::Ident(name) => {
                if let Some(ty) = self.type_env.lookup(name) {
                    ty
                } else {
                    self.engine.fresh_var()
                }
            }

            Expr::Binary { op, lhs, rhs } => {
                let lhs_ty = self.analyze_expr(lhs);
                let rhs_ty = self.analyze_expr(rhs);
                if op == "==" || op == "!=" || op == "<" || op == ">" || op == "<=" || op == ">=" || op == "&&" || op == "||" {
                    self.unify_or_error(&lhs_ty, &rhs_ty, &lhs.span());
                    Ty::Bool
                } else {
                    self.unify_or_error(&lhs_ty, &rhs_ty, &lhs.span());
                    lhs_ty
                }
            }

            Expr::Call { callee, args } => {
                for arg in args {
                    self.analyze_expr(arg);
                }
                if let Expr::Ident(name) = callee.as_ref() {
                    if name == "move_to_device" {
                        if let Some(first) = args.first() {
                            if let Expr::Ident(var) = first {
                                self.borrow_checker.move_to_device(var, "gpu");
                            }
                        }
                    }
                }
                self.engine.fresh_var()
            }

            Expr::If { cond, then_block, else_block } => {
                let cond_ty = self.analyze_expr(cond);
                self.unify_or_error(&Ty::Bool, &cond_ty, &cond.span());
                let hw = self.hw_context.clone();
                let then_ty = self.analyze_block(then_block, hw.as_ref());
                if let Some(eb) = else_block {
                    let else_ty = self.analyze_expr(eb);
                    self.unify_or_error(&then_ty, &else_ty, &then_block.span);
                }
                then_ty
            }

            Expr::Block(block) => {
                let hw = self.hw_context.clone();
                self.analyze_block(block, hw.as_ref())
            }

            Expr::Match { expr, arms } => {
                self.analyze_expr(expr);
                let mut result_ty = self.engine.fresh_var();
                for arm in arms {
                    let arm_ty = self.analyze_expr(&arm.expr);
                    self.unify_or_error(&result_ty, &arm_ty, &arm.expr.span());
                    result_ty = arm_ty;
                }
                result_ty
            }

            Expr::Cast { expr, ty } => {
                self.analyze_expr(expr);
                self.type_expr_to_ty(ty)
            }

            Expr::Spawn { task } => {
                self.analyze_expr(task);
                Ty::App { name: "JoinHandle".into(), args: vec![] }
            }

            Expr::ChanCreate { ty } => Ty::App { name: "Chan".into(), args: vec![self.type_expr_to_ty(ty)] },
            Expr::ChanSend { chan, value } => {
                self.analyze_expr(chan);
                self.analyze_expr(value);
                Ty::App { name: "ChanSendResult".into(), args: vec![] }
            }
            Expr::ChanRecv { chan } => {
                self.analyze_expr(chan);
                self.engine.fresh_var()
            }
            Expr::Select { arms } => {
                let hw = self.hw_context.clone();
                for arm in arms {
                    self.analyze_block(&arm.body, hw.as_ref());
                }
                self.engine.fresh_var()
            }

            Expr::For { var, iter, body } => {
                self.analyze_expr(iter);
                self.type_env.push_scope();
                self.type_env.insert(var, Ty::I32);
                let hw = self.hw_context.clone();
                let body_ty = self.analyze_block(body, hw.as_ref());
                self.type_env.pop_scope();
                body_ty
            }

            Expr::OwnershipExpr { inner, .. } => self.analyze_expr(inner),
            Expr::BorrowExpr { inner, is_mut, .. } => {
                let inner_ty = self.analyze_expr(inner);
                Ty::Ref { is_mut: *is_mut, inner: Box::new(inner_ty) }
            }
            Expr::LifetimeAnnotation { expr, .. } => self.analyze_expr(expr),
            Expr::Unsafe(block) => {
                let mut ty = self.engine.fresh_var();
                for stmt in &block.stmts {
                    if let Stmt::Expr(e) = stmt {
                        ty = self.analyze_expr(e);
                    }
                }
                ty
            }
        };

        let _ = self.type_table.insert(expr.span(), ty.clone());
        ty
    }

    /// Analyze a block, returning the type of the last expression (or unit).
    /// Also performs borrow checking when hw context is provided.
    fn analyze_block(&mut self, block: &Block, hw: Option<&HardwareAnnotation>) -> Ty {
        self.type_env.push_scope();
        let mut last_ty = Ty::App { name: "Unit".into(), args: vec![] };
        for stmt in &block.stmts {
            match stmt {
                Stmt::Let(v) => {
                    if let Some(init) = &v.init {
                        // Use-after-move check for identifier initializers
                        if let Expr::Ident(src_name) = init {
                            if self.borrow_checker.is_moved(src_name) {
                                self.diagnostics.push(Diagnostic {
                                    severity: Severity::Error,
                                    code: Some("E_USE_AFTER_MOVE".into()),
                                    message: format!("Use of moved value '{}'", src_name),
                                    span: Some(v.span.clone()),
                                    hint: Some("Value was moved to device; it is invalid on host.".into()),
                                    hardware_context: hw.map(|h| format!("{:?}", h.target)),
                                });
                            }
                        }

                        // Borrow checking for let initializers. (Calls that move,
                        // e.g. `move_to_device(x)`, are recorded once in
                        // analyze_expr below — recording here too produced a
                        // spurious second E_USE_AFTER_MOVE.)
                        match init {
                            Expr::BorrowExpr { inner, is_mut, .. } => {
                                if let Expr::Ident(name) = inner.as_ref() {
                                    let kind = if *is_mut { BorrowKind::Mut } else { BorrowKind::Shared };
                                    self.borrow_checker.borrow(name, kind);
                                }
                            }
                            _ => {}
                        }

                        let init_ty = self.analyze_expr(init);
                        if let Some(decl_ty) = &v.ty {
                            let expected = self.type_expr_to_ty(decl_ty);
                            self.unify_or_error(&expected, &init_ty, &v.span);
                        }
                        self.type_env.insert(&v.name, init_ty);
                    } else {
                        let fresh = self.engine.fresh_var();
                        self.type_env.insert(&v.name, fresh);
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
                Stmt::Const(c) => {
                    let val_ty = self.analyze_expr(&c.value);
                    if let Some(decl_ty) = &c.ty {
                        let expected = self.type_expr_to_ty(decl_ty);
                        self.unify_or_error(&expected, &val_ty, &c.span);
                    }
                    self.type_env.insert(&c.name, val_ty);
                }
                Stmt::Expr(e) => {
                    // Use-after-move check before type analysis
                    if let Expr::Ident(name) = e {
                        if self.borrow_checker.is_moved(name) {
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
                    if let Expr::Call { callee, .. } = e {
                        if let Expr::Ident(name) = callee.as_ref() {
                            if self.borrow_checker.is_moved(name) {
                                self.diagnostics.push(Diagnostic {
                                    severity: Severity::Error,
                                    code: Some("E_USE_AFTER_MOVE".into()),
                                    message: format!("Use of moved function '{}'", name),
                                    span: Some(block.span.clone()),
                                    hint: None,
                                    hardware_context: None,
                                });
                            }
                        }
                    }
                    last_ty = self.analyze_expr(e);
                }
                Stmt::Return(Some(e)) => {
                    last_ty = self.analyze_expr(e);
                }
                Stmt::Return(None) => {
                    last_ty = Ty::App { name: "Unit".into(), args: vec![] };
                }
                Stmt::Match { expr, arms } => {
                    self.analyze_expr(expr);
                    for arm in arms {
                        self.analyze_expr(&arm.expr);
                    }
                }
                Stmt::For { var, iter, body } => {
                    self.analyze_expr(iter);
                    self.type_env.push_scope();
                    self.type_env.insert(var, Ty::I32);
                    self.analyze_block(body, hw);
                    self.type_env.pop_scope();
                }
                Stmt::While { cond, body } => {
                    let cond_ty = self.analyze_expr(cond);
                    self.unify_or_error(&Ty::Bool, &cond_ty, &cond.span());
                    self.analyze_block(body, hw);
                }
                Stmt::Loop { body } => {
                    self.analyze_block(body, hw);
                }
                Stmt::Break(e) => {
                    if let Some(expr) = e {
                        self.analyze_expr(expr);
                    }
                }
                Stmt::Continue => {}
                Stmt::Assign { lhs, rhs } => {
                    self.analyze_expr(lhs);
                    self.analyze_expr(rhs);
                }
                Stmt::Unsafe(ub) => {
                    for s in &ub.stmts {
                        if let Stmt::Expr(e) = s {
                            last_ty = self.analyze_expr(e);
                        }
                    }
                }
            }
        }
        self.type_env.pop_scope();
        last_ty
    }
}
