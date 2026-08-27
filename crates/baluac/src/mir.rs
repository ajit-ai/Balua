//! Balua MIR — Mid-level Intermediate Representation (Section 2.1, Stage 4)
//! SSA form, hardware-annotated basic blocks. Carries target metadata per block.

use crate::ast::{HardwareAnnotation, HardwareTarget};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Terminator {
    Return(Option<String>),
    Jump(usize),
    Branch { cond: String, then_bb: usize, else_bb: usize },
    Unreachable,
}

pub struct MirBuilder;

impl MirBuilder {
    pub fn lower(program: &crate::ast::Program) -> Vec<MirModule> {
        let mut modules = Vec::new();
        for m in &program.modules {
            let mut mir_funcs = Vec::new();
            for item in &m.items {
                match item {
                    crate::ast::Item::FnDecl(f) => {
                        mir_funcs.push(MirFunction {
                            name: f.name.clone(),
                            hardware: f.hardware.as_ref().map(|h| h.target.clone()),
                            basic_blocks: vec![BasicBlock {
                                id: 0,
                                hardware: f.hardware.clone(),
                                instructions: vec![],
                                terminator: Terminator::Return(None),
                            }],
                            span: f.span.clone(),
                        });
                    }
                    crate::ast::Item::KernelDecl(k) => {
                        mir_funcs.push(MirFunction {
                            name: k.name.clone(),
                            hardware: Some(HardwareTarget::Gpu),
                            basic_blocks: vec![BasicBlock {
                                id: 0,
                                hardware: Some(k.annotation.clone()),
                                instructions: vec![Instruction::HwIntrinsic {
                                    name: "thread_id.x".into(),
                                    dest: Some("%tid_x".into()),
                                    args: vec![],
                                }],
                                terminator: Terminator::Return(None),
                            }],
                            span: k.span.clone(),
                        });
                    }
                    crate::ast::Item::CircuitDecl(c) => {
                        mir_funcs.push(MirFunction {
                            name: c.name.clone(),
                            hardware: Some(HardwareTarget::Fpga),
                            basic_blocks: vec![BasicBlock {
                                id: 0,
                                hardware: Some(c.annotation.clone()),
                                instructions: vec![Instruction::HwIntrinsic {
                                    name: "pipeline.II=1".into(),
                                    dest: None,
                                    args: vec![],
                                }],
                                terminator: Terminator::Return(None),
                            }],
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
