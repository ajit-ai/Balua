//! baluac — Balua compiler library
//! Implements Sections 2, 3, 5, 10 of the Balua Master Specification.
//! Front-end stages: Lexer → Parser → Semantic → MIR → Backends

pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod mir;
pub mod parser;
pub mod semantic;

pub mod backend;
pub mod linker;
pub mod safety;
pub mod types;

pub use diagnostics::Diagnostic;
