//! Borrow checker — Rust-style ownership adapted for Balua (Section 5.1-4)
//! Enforces single owner, move semantics, &T / &mut T, plus cross-device moves (CPU↔GPU transfers).

use crate::diagnostics::Diagnostic;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowKind { Shared, Mut }

#[derive(Debug)]
struct Loan { kind: BorrowKind, alive: bool }

pub struct BorrowChecker {
    loans: HashMap<String, Vec<Loan>>,
    moved: HashMap<String, String>, // var -> location moved to
    diagnostics: Vec<Diagnostic>,
}

impl BorrowChecker {
    pub fn new() -> Self { Self { loans: HashMap::new(), moved: HashMap::new(), diagnostics: vec![] } }

    /// Record a borrow: &T or &mut T
    pub fn borrow(&mut self, var: &str, kind: BorrowKind) {
        let entry = self.loans.entry(var.to_string()).or_default();
        // mutable borrow while shared borrows alive = error (E_BORROW_CONFLICT)
        if kind == BorrowKind::Mut && entry.iter().any(|l| l.alive) {
            self.diagnostics.push(Diagnostic::error(format!("Cannot borrow '{}' as mutable: already borrowed", var))
                .with_code("E_BORROW_CONFLICT")
                .with_hint("Shared borrows must end before mutable borrow."));
        }
        // shared borrow while mutable alive = error (E_BORROW_CONFLICT)
        if kind == BorrowKind::Shared && entry.iter().any(|l| l.alive && l.kind == BorrowKind::Mut) {
            self.diagnostics.push(Diagnostic::error(format!("Cannot borrow '{}' as shared: mutably borrowed", var))
                .with_code("E_BORROW_CONFLICT")
                .with_hint("Mutable borrow is exclusive."));
        }
        entry.push(Loan { kind, alive: true });
    }

    /// Move semantics: moving to GPU invalidates CPU handle
    pub fn move_to_device(&mut self, var: &str, target: &str) {
        if self.moved.contains_key(var) {
            self.diagnostics.push(Diagnostic::error(format!("Use of moved value '{}' — already moved to {}", var, self.moved[var]))
                .with_code("E_USE_AFTER_MOVE"));
        } else {
            self.moved.insert(var.to_string(), target.to_string());
        }
    }

    pub fn is_moved(&self, var: &str) -> bool { self.moved.contains_key(var) }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> { std::mem::take(&mut self.diagnostics) }
}
