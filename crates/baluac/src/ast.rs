//! Balua AST — Typed Abstract Syntax Tree (Section 2.1, Stage 2)
//! Node types: FnDecl, VarDecl, HardwareBlock, KernelDecl, CircuitDecl,
//! QuantumBlock, OwnershipExpr, BorrowExpr, LifetimeAnnotation, FFIDecl, UnsafeBlock

use crate::diagnostics::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Program {
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    FnDecl(FnDecl),
    VarDecl(VarDecl),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    TraitDecl(TraitDecl),
    ImplDecl(ImplDecl),
    UseDecl(UseDecl),
    HardwareBlock(HardwareBlock),
    KernelDecl(KernelDecl),
    CircuitDecl(CircuitDecl),
    QuantumBlock(QuantumBlock),
    FFIDecl(FFIDecl),
    UnsafeBlock(UnsafeBlock),
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub ret_ty: Option<TypeExpr>,
    pub hardware: Option<HardwareAnnotation>,
    pub is_async: bool,
    pub is_extern: Option<String>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub is_mut: bool,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub init: Option<Expr>,
    pub is_mut: bool,
    pub ownership: Ownership,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ownership {
    Owned,
    BorrowShared,
    BorrowMut,
    Moved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareAnnotation {
    pub target: HardwareTarget,
    pub params: Vec<(String, String)>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareTarget {
    Cpu,
    Gpu,
    Npu,
    Fpga,
    Quantum,
    Embedded,
}

#[derive(Debug, Clone)]
pub struct HardwareBlock {
    pub annotation: HardwareAnnotation,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct KernelDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub annotation: HardwareAnnotation,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CircuitDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<TypeExpr>,
    pub annotation: HardwareAnnotation,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct QuantumBlock {
    pub annotation: HardwareAnnotation,
    pub qubits: usize,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    pub methods: Vec<FnDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub trait_name: Option<String>,
    pub target: String,
    pub methods: Vec<FnDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UseDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FFIDecl {
    pub abi: String,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnsafeBlock {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(VarDecl),
    Expr(Expr),
    Return(Option<Expr>),
    Assign { lhs: Expr, rhs: Expr },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Binary { op: String, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    OwnershipExpr { inner: Box<Expr>, kind: Ownership },
    BorrowExpr { inner: Box<Expr>, is_mut: bool, lifetime: Option<String> },
    LifetimeAnnotation { name: String, expr: Box<Expr> },
    Unsafe(Box<UnsafeBlock>),
    Block(Block),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i128, String),   // value + suffix
    Float(f64, String),
    Str(String),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Primitive(String),
    Pointer { is_mut: bool, inner: Box<TypeExpr> },
    Reference { is_mut: bool, lifetime: Option<String>, inner: Box<TypeExpr> },
    Slice(Box<TypeExpr>),
    Array { inner: Box<TypeExpr>, size: usize },
    Generic { name: String, args: Vec<TypeExpr> },
    Hardware { base: String, params: Vec<String> },
    Qubit(Option<usize>),
    Simd { width: usize, inner: Box<TypeExpr> },
    Tensor { ty: Box<TypeExpr>, shape: Vec<usize> },
    Stream(Box<TypeExpr>),
    Function { params: Vec<TypeExpr>, ret: Box<TypeExpr> },
    Inferred,
}
