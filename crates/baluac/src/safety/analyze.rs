//! M2: MIR safety walkers for `--safety-profile` enforcement.
//!
//! The pre-M2 checkers (`misra.rs`, `wcet.rs`, `stack_analysis.rs`) take
//! caller-supplied facts, so on their own they analyze nothing. This module
//! derives those facts by walking MIR and feeds them back in, producing
//! `E_SAFETY_*` diagnostics:
//!
//! - heap: `Alloca` / `HwIntrinsic` / `MoveToDevice` instructions, or calls to
//!   exactly `move_to_device`, `box`, `alloc`, `malloc`, `calloc`, `realloc`.
//!   Matching is exact (not substring) so user functions like `allocate` are
//!   clean. Dynamic-lifetime ops.
//! - recursion: call-graph cycles, direct and indirect.
//! - unbounded loops: reachable CFG components with a cycle and no exit edge,
//!   computed via Kosaraju SCCs (linear time). Branch conditions defined by
//!   constant `const.N` are folded, so `while true` is correctly unbounded
//!   while `while false` is accepted. Non-constant conditions count as exits
//!   (structural analysis, not semantic).
//! - stack: `32 + 4 * (params + distinct dest registers)` bytes per function,
//!   limited by `#[max_stack(N)]` if present else `--safety-stack-limit`.
//! - WCET: `#[wcet_cycles(N)]` bound against an instruction-count estimate
//!   (documented-crude structural proxy, not a timing model) → `E_SAFETY_WCET`.
//! - policy: enforcement is uniform across `Safe`/`Unsafe`/`Trusted` tiers —
//!   a profile describes code guarantees, not function labels. Hardware-
//!   targeted functions (`@hw::gpu/fpga/npu/quantum/embedded`, i.e. anything
//!   but `@hw::cpu`/plain) are skipped: the CPU profile does not govern HW
//!   regions (own backends/validators, post-GA). Skipped names are reported.
//!
//! `formal_verification.rs` stays emit-only (post-GA scope).

use crate::diagnostics::{Diagnostic, Severity};
use crate::mir::{Instruction, MirFunction, MirModule, Terminator};
use crate::safety::misra::{SafetyChecker, SafetyProfile};
use crate::safety::stack_analysis::StackAnalyzer;
use crate::safety::wcet::WcetAnalyzer;
use crate::ast::HardwareTarget;
use std::collections::{HashMap, HashSet};

/// Outcome of a safety check run.
pub struct SafetyReport {
    pub errors: Vec<Diagnostic>,
    pub stack_report: String,
    pub checked_functions: usize,
    /// Hardware-targeted functions skipped (profile governs CPU code only).
    pub skipped_hw: Vec<String>,
}

/// Parse a `--safety-profile` value. `None` means unknown (fail-closed);
/// `Some(SafetyProfile::None)` (i.e. `none`) means no enforcement.
pub fn parse_profile(s: &str) -> Option<SafetyProfile> {
    match s.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
        "autosarcpp14" | "autosar" => Some(SafetyProfile::AutosarCpp14),
        "do178c" => Some(SafetyProfile::Do178C),
        "iec61508" => Some(SafetyProfile::Iec61508),
        "none" => Some(SafetyProfile::None),
        _ => None,
    }
}

/// Diagnostic for an unknown profile name (fail-closed).
pub fn unknown_profile_error(name: &str) -> Diagnostic {
    Diagnostic::error(format!(
        "unknown safety profile '{}' (expected AUTOSAR_CPP14|DO-178C|IEC61508|none)",
        name
    ))
    .with_code("E_SAFETY_PROFILE")
}

/// Run all safety checks over lowered MIR modules.
pub fn check_modules(
    modules: &[MirModule],
    profile: &SafetyProfile,
    stack_limit: Option<usize>,
) -> SafetyReport {
    let mut report = SafetyReport { errors: vec![], stack_report: String::new(), checked_functions: 0, skipped_hw: vec![] };
    if *profile == SafetyProfile::None {
        return report;
    }
    // CPU profile governs CPU code only; HW regions have their own backends.
    let fns: Vec<&MirFunction> = modules
        .iter()
        .flat_map(|m| m.functions.iter())
        .filter(|f| match &f.hardware {
            None | Some(HardwareTarget::Cpu) => true,
            Some(_) => {
                report.skipped_hw.push(f.name.clone());
                false
            }
        })
        .collect();
    report.checked_functions = fns.len();
    let checker = SafetyChecker::new(profile.clone());
    let edges = call_graph(&fns);
    let recursive = recursive_set(&fns, &edges);
    let mut stack = StackAnalyzer::new();
    let mut wcet = WcetAnalyzer::new();
    for f in &fns {
        let heap = uses_heap(f);
        let rec = recursive.contains(&f.name);
        let unbounded = has_unbounded_loop(f);
        // Reuse the profile checker's messages, fed with MIR-derived facts.
        if heap {
            for m in checker.check_function(&f.name, true, false, false) {
                report.errors.push(safety_error("E_SAFETY_HEAP", m, f));
            }
        }
        if rec {
            for m in checker.check_function(&f.name, false, true, false) {
                report.errors.push(safety_error("E_SAFETY_RECURSION", m, f));
            }
        }
        if unbounded {
            for m in checker.check_function(&f.name, false, false, true) {
                report.errors.push(safety_error("E_SAFETY_LOOP", m, f));
            }
        }
        let (attr_stack, attr_wcet) = fn_attr_limits(f);
        stack.record(&f.name, estimate_stack(f), attr_stack.or(stack_limit));
        if let Some(bound) = attr_wcet {
            wcet.annotate(&f.name, bound);
            wcet.estimate(&f.name, instruction_count(f) as u64);
        }
    }
    for (func, usage, limit) in stack.overflows() {
        let span = fns.iter().find(|f| f.name == func).map(|f| f.span.clone());
        let mut d = Diagnostic::error(format!("Stack overflow: '{}' uses {} bytes but limit is {} bytes", func, usage, limit))
            .with_code("E_SAFETY_STACK");
        if let Some(s) = span {
            d = d.with_span(s);
        }
        report.errors.push(d);
    }
    for (func, est, bound) in wcet.overflows() {
        let span = fns.iter().find(|f| f.name == func).map(|f| f.span.clone());
        let mut d = Diagnostic::error(format!("WCET violation: '{}' estimated {} cycles exceeds bound {} cycles", func, est, bound))
            .with_code("E_SAFETY_WCET");
        if let Some(s) = span {
            d = d.with_span(s);
        }
        report.errors.push(d);
    }
    report.stack_report = stack.report();
    if !report.skipped_hw.is_empty() {
        report.stack_report.push_str(&format!("skipped hardware-targeted functions: {}\n", report.skipped_hw.join(", ")));
    }
    report
}

/// `#[max_stack(N)]` / `#[wcet_cycles(N)]` limits from raw annotation lexemes
/// like `#[max_stack(512)]`. Unknown/malformed attributes yield `None`
/// (semantic already warns `W_UNKNOWN_ATTR`).
fn fn_attr_limits(f: &MirFunction) -> (Option<usize>, Option<u64>) {
    let mut stack = None;
    let mut wcet = None;
    for attr in &f.attrs {
        let inner = attr.trim_start_matches("#[").trim_end_matches(']');
        if let Some((name, rest)) = inner.split_once('(') {
            let arg = rest.trim_end_matches(')');
            match name.trim() {
                "max_stack" => {
                    if let Ok(n) = arg.trim().parse::<usize>() {
                        stack = Some(n);
                    }
                }
                "wcet_cycles" => {
                    if let Ok(n) = arg.trim().parse::<u64>() {
                        wcet = Some(n);
                    }
                }
                _ => {}
            }
        }
    }
    (stack, wcet)
}

fn safety_error(code: &str, message: String, f: &MirFunction) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: Some(code.into()),
        message,
        span: Some(f.span.clone()),
        hint: Some("Safety profile forbids this construct; restructure without heap/recursion/unbounded loops.".into()),
        hardware_context: None,
    }
}

/// Dynamic-lifetime operations: heap allocation or device transfer.
fn uses_heap(f: &MirFunction) -> bool {
    f.basic_blocks.iter().flat_map(|b| b.instructions.iter()).any(|i| match i {
        Instruction::Alloca { .. } | Instruction::HwIntrinsic { .. } | Instruction::MoveToDevice { .. } => true,
        Instruction::Call { callee, .. } => is_heap_call(callee),
        _ => false,
    })
}

fn is_heap_call(callee: &str) -> bool {
    // Exact match only: substring matching misflagged user functions like
    // `allocate`. The language is case-sensitive, so no lowercasing.
    matches!(callee.trim_start_matches(['@', '%']), "move_to_device" | "box" | "alloc" | "malloc" | "calloc" | "realloc")
}

/// Call graph restricted to defined functions.
fn call_graph(fns: &[&MirFunction]) -> HashMap<String, Vec<String>> {
    let defined: HashSet<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for f in fns {
        let mut callees = Vec::new();
        for bb in &f.basic_blocks {
            for inst in &bb.instructions {
                if let Instruction::Call { callee, .. } = inst {
                    let name = callee.trim_start_matches(['@', '%']).to_string();
                    if defined.contains(name.as_str()) {
                        callees.push(name);
                    }
                }
            }
        }
        edges.insert(f.name.clone(), callees);
    }
    edges
}

/// Functions that can reach themselves (direct or indirect recursion).
fn recursive_set(fns: &[&MirFunction], edges: &HashMap<String, Vec<String>>) -> HashSet<String> {
    fns.iter()
        .map(|f| f.name.clone())
        .filter(|name| reaches(name, name, edges, &mut HashSet::new(), true))
        .collect()
}

fn reaches(
    cur: &str,
    target: &str,
    edges: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    first: bool,
) -> bool {
    if !first && cur == target {
        return true;
    }
    if !visited.insert(cur.to_string()) {
        return false;
    }
    if let Some(next) = edges.get(cur) {
        for n in next {
            if reaches(n, target, edges, visited, false) {
                return true;
            }
        }
    }
    false
}

/// True if the function contains a reachable CFG cycle with no exit edge.
/// Computed over Kosaraju SCCs (linear time). Branch conditions defined by a
/// constant (`const.N`: nonzero = taken, zero = not taken) are folded, so
/// `while true` is unbounded and `while false` is accepted.
fn has_unbounded_loop(f: &MirFunction) -> bool {
    // Registers defined by integer constants.
    let mut consts: HashMap<String, Option<bool>> = HashMap::new();
    for bb in &f.basic_blocks {
        for inst in &bb.instructions {
            if let Instruction::BinOp { dest, op, .. } = inst {
                if let Some(n) = op.strip_prefix("const.") {
                    // Note: "constf."/"consts." do not match "const." prefix
                    // followed by an integer; parse failure means unknown.
                    consts.insert(dest.clone(), n.parse::<i64>().map(|v| v != 0).ok());
                }
            }
        }
    }
    let succ_of = |term: &Terminator| -> Vec<usize> {
        match term {
            Terminator::Jump(t) => vec![*t],
            Terminator::Branch { cond, then_bb, else_bb } => match consts.get(cond) {
                Some(Some(true)) => vec![*then_bb],
                Some(Some(false)) => vec![*else_bb],
                _ => vec![*then_bb, *else_bb],
            },
            Terminator::Return(_) | Terminator::Unreachable => vec![],
        }
    };
    let succ: HashMap<usize, Vec<usize>> = f
        .basic_blocks
        .iter()
        .map(|b| (b.id, succ_of(&b.terminator)))
        .collect();
    // Reachable-from-entry subgraph (block 0).
    let mut reachable = HashSet::new();
    let mut work = vec![0usize];
    while let Some(n) = work.pop() {
        if reachable.insert(n) {
            if let Some(next) = succ.get(&n) {
                work.extend(next.iter().copied());
            }
        }
    }
    // Kosaraju pass 1: finish order (iterative).
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    for &n in &reachable {
        if visited.contains(&n) {
            continue;
        }
        let mut stack = vec![(n, false)];
        while let Some((v, processed)) = stack.pop() {
            if processed {
                order.push(v);
                continue;
            }
            if !visited.insert(v) {
                continue;
            }
            stack.push((v, true));
            if let Some(next) = succ.get(&v) {
                for &w in next {
                    if reachable.contains(&w) && !visited.contains(&w) {
                        stack.push((w, false));
                    }
                }
            }
        }
    }
    // Transpose graph.
    let mut trans: HashMap<usize, Vec<usize>> = HashMap::new();
    for (&u, vs) in &succ {
        if !reachable.contains(&u) {
            continue;
        }
        for &v in vs {
            if reachable.contains(&v) {
                trans.entry(v).or_default().push(u);
            }
        }
    }
    // Pass 2: components in reverse finish order.
    let mut comp_of: HashMap<usize, usize> = HashMap::new();
    let mut comps: Vec<Vec<usize>> = Vec::new();
    for &n in order.iter().rev() {
        if comp_of.contains_key(&n) {
            continue;
        }
        let id = comps.len();
        let mut comp = Vec::new();
        let mut stack = vec![n];
        while let Some(v) = stack.pop() {
            if comp_of.contains_key(&v) {
                continue;
            }
            comp_of.insert(v, id);
            comp.push(v);
            if let Some(prev) = trans.get(&v) {
                for &w in prev {
                    if !comp_of.contains_key(&w) {
                        stack.push(w);
                    }
                }
            }
        }
        comps.push(comp);
    }
    // A component is cyclic if it has >1 node or a self-edge; it is
    // unbounded if no member has a successor outside the component.
    let comp_set: Vec<HashSet<usize>> = comps.iter().map(|c| c.iter().copied().collect()).collect();
    for (comp, set) in comps.iter().zip(comp_set.iter()) {
        let cyclic = comp.len() > 1
            || comp.first().map(|n| succ.get(n).map(|next| next.contains(n)).unwrap_or(false)).unwrap_or(false);
        if !cyclic {
            continue;
        }
        let has_exit = set.iter().any(|n| {
            succ.get(n).map(|next| next.iter().any(|s| !set.contains(s))).unwrap_or(false)
        });
        if !has_exit {
            return true;
        }
    }
    false
}

/// Number of MIR instructions (documented-crude WCET estimate basis).
fn instruction_count(f: &MirFunction) -> usize {
    f.basic_blocks.iter().map(|b| b.instructions.len()).sum()
}

/// Rough frame estimate: base + 4 bytes per param and distinct dest register.
fn estimate_stack(f: &MirFunction) -> usize {
    let mut dests = HashSet::new();
    for bb in &f.basic_blocks {
        for inst in &bb.instructions {
            match inst {
                Instruction::Alloca { dest, .. }
                | Instruction::Load { dest, .. }
                | Instruction::BinOp { dest, .. }
                | Instruction::MoveToDevice { dest, .. }
                | Instruction::Phi { dest, .. } => {
                    dests.insert(dest.clone());
                }
                Instruction::Call { dest, .. } => {
                    if let Some(d) = dest {
                        dests.insert(d.clone());
                    }
                }
                Instruction::HwIntrinsic { dest, .. } => {
                    if let Some(d) = dest {
                        dests.insert(d.clone());
                    }
                }
                Instruction::Store { .. } => {}
            }
        }
    }
    32 + 4 * (f.params.len() + dests.len())
}
