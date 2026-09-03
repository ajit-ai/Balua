//! Balua MIR — Mid-level Intermediate Representation (Section 2.1, Stage 4)
//! SSA-lowered representation with hardware-annotated basic blocks.
//! Phase 2: MirBuilder now lowers real function bodies into instructions.

use crate::ast::{Block, Expr, HardwareAnnotation, HardwareTarget, Item, Literal, Program, Stmt};
use crate::diagnostics::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirModule {
    pub name: String,
    pub functions: Vec<MirFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub name: String,
    pub hardware: Option<HardwareTarget>,
    pub params: Vec<(String, String)>,
    pub basic_blocks: Vec<BasicBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: usize,
    pub hardware: Option<HardwareAnnotation>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// %dest = alloca T
    Alloca { dest: String, ty: String },
    /// %dest = load %src
    Load { dest: String, src: String },
    /// store %val -> %ptr
    Store { val: String, ptr: String },
    /// %dest = binop %lhs, %rhs
    BinOp { dest: String, op: String, lhs: String, rhs: String },
    /// %dest = call @fn(args...)
    Call { dest: Option<String>, callee: String, args: Vec<String> },
    /// hardware intrinsic
    HwIntrinsic { dest: Option<String>, name: String, args: Vec<String> },
    /// move_to_device %src -> %dest (CPU->GPU transfer)
    MoveToDevice { dest: String, src: String },
    /// phi node for SSA
    Phi { dest: String, incoming: Vec<(String, usize)> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Terminator {
    Return(Option<String>),
    Jump(usize),
    Branch { cond: String, then_bb: usize, else_bb: usize },
    Unreachable,
}

pub struct MirBuilder;

impl MirBuilder {
    pub fn lower(program: &Program) -> Vec<MirModule> {
        let mut modules = Vec::new();
        for m in &program.modules {
            let mut mir_funcs = Vec::new();
            for item in &m.items {
                match item {
                    Item::FnDecl(f) => {
                        let params = f.params.iter().map(|p| (p.name.clone(), ty_name(&p.ty))).collect();
                        mir_funcs.push(MirFunction {
                            name: f.name.clone(),
                            hardware: f.hardware.as_ref().map(|h| h.target.clone()),
                            params,
                            basic_blocks: Lower::lower_fn(f.name.as_str(), &f.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>(), &f.body),
                            span: f.span.clone(),
                        });
                    }
                    Item::KernelDecl(k) => {
                        mir_funcs.push(MirFunction {
                            name: k.name.clone(),
                            hardware: Some(HardwareTarget::Gpu),
                            params: k.params.iter().map(|p| (p.name.clone(), ty_name(&p.ty))).collect(),
                            basic_blocks: Lower::unsupported(&k.body, "@hw::gpu kernel"),
                            span: k.span.clone(),
                        });
                    }
                    Item::CircuitDecl(c) => {
                        mir_funcs.push(MirFunction {
                            name: c.name.clone(),
                            hardware: Some(HardwareTarget::Fpga),
                            params: c.params.iter().map(|p| (p.name.clone(), ty_name(&p.ty))).collect(),
                            basic_blocks: Lower::unsupported(&c.body, "@hw::fpga circuit"),
                            span: c.span.clone(),
                        });
                    }
                    _ => {}
                }
            }
            modules.push(MirModule { name: m.name.clone(), functions: mir_funcs });
        }
        modules
    }

    pub fn to_json(modules: &[MirModule]) -> String {
        serde_json::to_string_pretty(modules).unwrap_or_else(|_| "[]".into())
    }
}

pub fn ty_name(ty: &crate::ast::TypeExpr) -> String {
    match ty {
        crate::ast::TypeExpr::Primitive(p) => p.clone(),
        crate::ast::TypeExpr::Inferred => "_".to_string(),
        other => format!("{:?}", other),
    }
}

/// Lowers a function body (or bare-expression body) into MIR basic blocks.
pub struct Lower {
    /// virtual-register counter: %1, %2, ...
    next_tmp: usize,
    /// %name -> %reg for locals that map to a stable SSA value
    env: std::collections::HashMap<String, String>,
    blocks: Vec<BasicBlock>,
    current: usize,
    /// stack of open loop continue targets
    loop_stack: Vec<LoopCtx>,
}

struct LoopCtx {
    cond_bb: usize,
    next_bb: usize,
}

static UNSUPPORTED: &str = "E_UNSUPPORTED_ON_TARGET";

impl Lower {
    pub fn lower_fn(
        _name: &str,
        param_names: &[String],
        body: &Option<Block>,
    ) -> Vec<BasicBlock> {
        let mut l = Lower {
            next_tmp: 1,
            env: std::collections::HashMap::new(),
            blocks: vec![],
            current: 0,
            loop_stack: vec![],
        };
        // Bind parameters to stable %p<i> registers so Ident(param) lowers to
        // the incoming value (codegen binds the same register to the block param).
        for (i, pname) in param_names.iter().enumerate() {
            l.env.insert(pname.clone(), format!("%p{}", i));
        }
        l.new_block();
        match body {
            None => l.blocks[0].terminator = Terminator::Return(None),
            Some(b) => {
                let tail = l.lower_block_stmts(b);
                // If the last statement is a bare expression it serves as the
                // return value (already lowered+returned by lower_block_stmts).
                if let Some(v) = tail {
                    l.blocks[l.current].terminator = Terminator::Return(Some(v));
                } else if matches!(l.blocks[l.current].terminator, Terminator::Unreachable) {
                    // unreachable already set for await/unsupported
                } else if l.blocks[l.current].terminator == Terminator::Return(None)
                    && l.blocks[l.current].instructions.is_empty()
                {
                    // leave as return none
                } else if !matches!(l.blocks[l.current].terminator, Terminator::Return(_))
                    && !matches!(l.blocks[l.current].terminator, Terminator::Jump(_))
                    && !matches!(l.blocks[l.current].terminator, Terminator::Branch { .. })
                {
                    if !matches!(l.blocks[l.current].terminator, Terminator::Unreachable) {
                        l.blocks[l.current].terminator = Terminator::Return(None);
                    }
                }
            }
        }
        l.blocks
    }

    pub fn unsupported(_body: &Block, _why: &str) -> Vec<BasicBlock> {
        let mut b = BasicBlock {
            id: 0,
            hardware: None,
            instructions: vec![Instruction::HwIntrinsic {
                dest: Some("%err".into()),
                name: UNSUPPORTED.into(),
                args: vec![],
            }],
            terminator: Terminator::Unreachable,
        };
        b.terminator = Terminator::Unreachable;
        vec![b]
    }

    fn new_block(&mut self) {
        self.blocks.push(BasicBlock {
            id: self.blocks.len(),
            hardware: None,
            instructions: vec![],
            terminator: Terminator::Unreachable,
        });
        self.current = self.blocks.len() - 1;
    }

    fn fresh(&mut self) -> String {
        let r = format!("%{}", self.next_tmp);
        self.next_tmp += 1;
        r
    }

    fn emit(&mut self, inst: Instruction) {
        self.blocks[self.current].instructions.push(inst);
    }

    fn set_term(&mut self, t: Terminator) {
        self.blocks[self.current].terminator = t;
    }

    fn join(&mut self, bb: usize) {
        if self.blocks[self.current].terminator == Terminator::Unreachable {
            self.set_term(Terminator::Jump(bb));
        }
    }

    fn lower_block_stmts(&mut self, b: &Block) -> Option<String> {
        let n = b.stmts.len();
        for (i, stmt) in b.stmts.iter().enumerate() {
            let is_tail_expr = i + 1 == n && matches!(stmt, Stmt::Expr(_));
            match stmt {
                Stmt::Let(vd) => {
                    if let Some(init) = &vd.init {
                        let v = self.lower_expr(init);
                        self.env.insert(vd.name.clone(), v);
                    }
                }
                Stmt::Expr(e) => {
                    if is_tail_expr {
                        // The block's value is its trailing bare expression.
                        return Some(self.lower_expr(e));
                    }
                    self.lower_expr(e);
                }
                Stmt::Return(Some(e)) => {
                    let v = self.lower_expr(e);
                    self.set_term(Terminator::Return(Some(v)));
                    self.start_dead_block();
                }
                Stmt::Return(None) => {
                    self.set_term(Terminator::Return(None));
                    self.start_dead_block();
                }
                Stmt::Assign { lhs, rhs } => {
                    let v = self.lower_expr(rhs);
                    if let Expr::Ident(name) = lhs {
                        if let Some(reg) = self.env.get(name).cloned() {
                            // Write the updated value back into the variable's
                            // stable register so loop back-edges observe it.
                            self.emit(Instruction::BinOp { dest: reg.clone(), op: "id".into(), lhs: v.clone(), rhs: v });
                        } else {
                            self.env.insert(name.clone(), v);
                        }
                    }
                }
                Stmt::While { cond, body } => {
                    self.lower_while(cond, body);
                }
                Stmt::For { var, iter, body } => {
                    self.lower_for(var, iter, body);
                }
                Stmt::Loop { body } => {
                    self.lower_loop(body);
                }
                Stmt::Break(Some(e)) => {
                    let v = self.lower_expr(e);
                    let _ = v;
                    self.set_term(Terminator::Jump(self.loop_stack.last().map(|l| l.next_bb).unwrap_or(0)));
                    self.start_dead_block();
                }
                Stmt::Break(None) => {
                    self.set_term(Terminator::Jump(self.loop_stack.last().map(|l| l.next_bb).unwrap_or(0)));
                    self.start_dead_block();
                }
                Stmt::Continue => {
                    let target = self.loop_stack.last().map(|l| l.cond_bb).unwrap_or(0);
                    self.set_term(Terminator::Jump(target));
                    self.start_dead_block();
                }
                Stmt::Unsafe(ub) => {
                    for s in &ub.stmts {
                        // recurse through the unsafe block's statements
                        match s {
                            Stmt::Let(vd) => {
                                if let Some(init) = &vd.init {
                                    let v = self.lower_expr(init);
                                    self.env.insert(vd.name.clone(), v);
                                }
                            }
                            Stmt::Expr(e) => {
                                self.lower_expr(e);
                            }
                            Stmt::Return(Some(e)) => {
                                let v = self.lower_expr(e);
                                self.set_term(Terminator::Return(Some(v)));
                                self.start_dead_block();
                            }
                            Stmt::Return(None) => {
                                self.set_term(Terminator::Return(None));
                                self.start_dead_block();
                            }
                            _ => {}
                        }
                    }
                }
                _ => { /* unsupported stmt: skip (leave terminator as-is) */ }
            }
        }
        None
    }

    fn start_dead_block(&mut self) {
        // Start a fresh block after an unconditional terminator.
        self.new_block();
    }

    fn lower_block_contents(&mut self, b: &Block) -> Option<String> {
        // Delegates to the full statement lowerer so Assign/Unsafe etc. are
        // handled identically in if/while/for bodies and at the top level.
        self.lower_block_stmts(b)
    }

    fn lower_cond(&mut self, cond: &Expr) -> String {
        self.lower_expr(cond)
    }

    fn lower_while(&mut self, cond: &Expr, body: &Block) {
        let cond_bb = self.blocks.len();
        let body_bb = self.blocks.len() + 1;
        let next_bb = self.blocks.len() + 2;

        self.loop_stack.push(LoopCtx { cond_bb, next_bb });

        // if the current block is reachable, jump to cond
        self.join(cond_bb);
        // Pre-create cond, body, next in fixed order so their ids stay stable
        // even when the body contains nested constructs (e.g. if) that append
        // further blocks afterwards.
        self.new_block(); // cond = L
        self.new_block(); // body = L+1
        self.new_block(); // next = L+2
        self.current = cond_bb;
        let c = self.lower_cond(cond);
        self.set_term(Terminator::Branch { cond: c, then_bb: body_bb, else_bb: next_bb });
        self.current = body_bb;
        self.lower_block_contents(body);
        self.join(cond_bb);
        self.current = next_bb;
        self.loop_stack.pop();
    }

    fn lower_for(&mut self, var: &str, iter: &Expr, body: &Block) {
        // Support numeric ranges: `for var in start..end` lowers to:
        //   cnt = start
        //   cond: if cnt < end goto body else goto next
        //   body: <var := cnt; run body>; cnt = cnt + 1; goto cond
        //   next: continue
        // This gives correct bounded iteration (no infinite loop) and keeps
        // `break`/`continue` meaningful.
        let (start, end) = match iter {
            Expr::Binary { op, lhs, rhs } if op == ".." => {
                let s = self.lower_cond(lhs);
                let e = self.lower_cond(rhs);
                (s, e)
            }
            _ => {
                // Non-range iterator: bind the loop var to a default value and
                // run the body once (best-effort, matching prior minimal support).
                let z = self.fresh();
                self.emit(Instruction::BinOp { dest: z.clone(), op: "const.0".into(), lhs: z.clone(), rhs: z.clone() });
                self.env.insert(var.to_string(), z);
                let mut fake = Block { stmts: body.stmts.clone(), span: body.span.clone() };
                let cond = Expr::Literal(Literal::Bool(false));
                self.lower_while(&cond, &mut fake);
                return;
            }
        };

        // counter register (the loop variable is bound to this same register)
        let cnt = self.fresh();
        self.emit(Instruction::BinOp { dest: cnt.clone(), op: "id".into(), lhs: start.clone(), rhs: start });

        let cond_bb = self.blocks.len();
        let body_bb = cond_bb + 1;
        let next_bb = cond_bb + 2;
        self.loop_stack.push(LoopCtx { cond_bb, next_bb });
        self.join(cond_bb);
        // Pre-create cond/body/next in fixed order for stable ids.
        self.new_block(); // cond = L
        self.new_block(); // body = L+1
        self.new_block(); // next = L+2

        self.current = cond_bb;
        let cmp = self.fresh();
        self.emit(Instruction::BinOp { dest: cmp.clone(), op: "<".into(), lhs: cnt.clone(), rhs: end.clone() });
        self.set_term(Terminator::Branch { cond: cmp, then_bb: body_bb, else_bb: next_bb });

        self.current = body_bb;
        self.env.insert(var.to_string(), cnt.clone());
        self.lower_block_contents(body);
        // increment the counter, then fall through back to cond
        let one = self.fresh();
        self.emit(Instruction::BinOp { dest: one.clone(), op: "const.1".into(), lhs: one.clone(), rhs: one.clone() });
        let next_cnt = self.fresh();
        self.emit(Instruction::BinOp { dest: next_cnt.clone(), op: "+".into(), lhs: cnt.clone(), rhs: one });
        self.emit(Instruction::BinOp { dest: cnt.clone(), op: "id".into(), lhs: next_cnt.clone(), rhs: next_cnt });
        self.join(cond_bb);

        self.current = next_bb;
        self.loop_stack.pop();
    }

    fn lower_loop(&mut self, body: &Block) {
        let cond_bb = self.blocks.len();
        let body_bb = self.blocks.len() + 1;
        let next_bb = self.blocks.len() + 2;
        self.loop_stack.push(LoopCtx { cond_bb, next_bb });
        self.join(cond_bb);
        // Pre-create cond, body, next in fixed order so their ids stay stable
        // even with nested constructs in the body.
        self.new_block(); // cond = L
        self.new_block(); // body = L+1
        self.new_block(); // next = L+2
        self.blocks[cond_bb].terminator = Terminator::Jump(body_bb);
        self.current = body_bb;
        self.lower_block_contents(body);
        self.join(cond_bb);
        self.current = next_bb;
        self.loop_stack.pop();
    }

    fn lower_expr(&mut self, e: &Expr) -> String {
        match e {
            Expr::Literal(lit) => {
                let r = self.fresh();
                match lit {
                    Literal::Int(v, _) => {
                        self.emit(Instruction::BinOp { dest: r.clone(), op: format!("const.{}", v), lhs: r.clone(), rhs: r.clone() });
                    }
                    Literal::Bool(b) => {
                        let n = if *b { 1 } else { 0 };
                        self.emit(Instruction::BinOp { dest: r.clone(), op: format!("const.{}", n), lhs: r.clone(), rhs: r.clone() });
                    }
                    Literal::Float(fv, _) => {
                        self.emit(Instruction::BinOp { dest: r.clone(), op: format!("constf.{}", fv), lhs: r.clone(), rhs: r.clone() });
                    }
                    Literal::Str(s) => {
                        // emitted as a special const; codegen maps to a string constant
                        self.emit(Instruction::BinOp { dest: r.clone(), op: format!("consts.{:?}", s), lhs: r.clone(), rhs: r.clone() });
                    }
                }
                r
            }
            Expr::Ident(name) => {
                if let Some(v) = self.env.get(name) {
                    v.clone()
                } else {
                    let r = self.fresh();
                    r
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                let l = self.lower_expr(lhs);
                let r = self.lower_expr(rhs);
                let d = self.fresh();
                self.emit(Instruction::BinOp { dest: d.clone(), op: op.clone(), lhs: l, rhs: r });
                d
            }
            Expr::Call { callee, args } => {
                let callee_name = match callee.as_ref() {
                    Expr::Ident(n) => n.clone(),
                    _ => return self.fresh(),
                };
                let arg_regs: Vec<String> = args.iter().map(|a| self.lower_expr(a)).collect();
                let d = self.fresh();
                self.emit(Instruction::Call { dest: Some(d.clone()), callee: callee_name, args: arg_regs });
                d
            }
            Expr::Cast { expr, .. } => self.lower_expr(expr),
            Expr::Block(b) => {
                self.lower_block_stmts(b).unwrap_or_else(|| {
                    let r = self.fresh();
                    r
                })
            }
            Expr::If { cond, then_block, else_block } => {
                // Pre-create then/else/join in fixed order so the join id stays
                // stable even when a branch terminates early (break/return)
                // which appends extra dead blocks.
                let res = self.fresh();
                let then_bb = self.blocks.len();
                let else_bb = self.blocks.len() + 1;
                let join_bb = self.blocks.len() + 2;
                let c = self.lower_cond(cond);
                self.set_term(Terminator::Branch { cond: c, then_bb, else_bb });
                self.new_block(); // then = then_bb
                self.new_block(); // else = else_bb
                self.new_block(); // join = join_bb
                self.current = then_bb;
                if let Some(tv) = self.lower_block_contents(then_block) {
                    self.emit(Instruction::BinOp { dest: res.clone(), op: "id".into(), lhs: tv.clone(), rhs: tv });
                }
                self.join(join_bb);
                self.current = else_bb;
                match else_block {
                    None => { self.join(join_bb); }
                    Some(ex) => match ex.as_ref() {
                        Expr::Block(eb) => {
                            if let Some(ev) = self.lower_block_contents(eb) {
                                self.emit(Instruction::BinOp { dest: res.clone(), op: "id".into(), lhs: ev.clone(), rhs: ev });
                            }
                            self.join(join_bb);
                        }
                        Expr::If { .. } => {
                            let ev = self.lower_expr(ex.as_ref());
                            self.emit(Instruction::BinOp { dest: res.clone(), op: "id".into(), lhs: ev.clone(), rhs: ev });
                            self.join(join_bb);
                        }
                        _ => { self.join(join_bb); }
                    },
                }
                self.join(join_bb);
                self.current = join_bb;
                res
            }
            _ => self.fresh(),
        }
    }
}
