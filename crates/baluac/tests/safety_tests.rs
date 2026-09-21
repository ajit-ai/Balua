//! M2 exit-criteria tests for safety enforcement.
//!
//! Each case lowers a Balua snippet to MIR and runs the safety walkers
//! (`safety::check_modules` under AUTOSAR_CPP14). Semantic diagnostics are
//! intentionally ignored here: these tests assert on `E_SAFETY_*` codes only.
//! (Note: `move_to_device` fixtures also produce a pre-existing semantic
//! `E_USE_AFTER_MOVE` via double recording in `semantic.rs`; that quirk is
//! out of M2 scope and not asserted here.)

use baluac_lib::safety::{check_modules, parse_profile, unknown_profile_error};
use baluac_lib::safety::misra::SafetyProfile;

fn check_src(src: &str) -> Vec<baluac_lib::diagnostics::Diagnostic> {
    check_src_with_stack(src, None).0
}

fn check_src_with_stack(
    src: &str,
    stack_limit: Option<usize>,
) -> (Vec<baluac_lib::diagnostics::Diagnostic>, String) {
    let report = check_full(src, stack_limit);
    let errors = report.errors;
    (errors, report.stack_report)
}

fn check_full(src: &str, stack_limit: Option<usize>) -> baluac_lib::safety::SafetyReport {
    let mut lexer = baluac_lib::lexer::Lexer::new(src, "test.bl");
    let (tokens, _) = lexer.tokenize();
    let mut parser = baluac_lib::parser::Parser::new(tokens);
    let (program, _) = parser.parse_program();
    let mut analyzer = baluac_lib::semantic::SemanticAnalyzer::new();
    let (_, type_table) = analyzer.analyze(&program);
    let mir = baluac_lib::mir::MirBuilder::lower_with_types(&program, &type_table);
    check_modules(&mir, &SafetyProfile::AutosarCpp14, stack_limit)
}

fn has_code(diags: &[baluac_lib::diagnostics::Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code.as_deref() == Some(code) && matches!(d.severity, baluac_lib::diagnostics::Severity::Error))
}

#[test]
fn heap_transfer_rejected() {
    let (errs, _) = check_src_with_stack(
        "fn main() -> i32 { let x = 5; let b = move_to_device(x, \"gpu\"); 0 }",
        None,
    );
    assert!(has_code(&errs, "E_SAFETY_HEAP"), "expected E_SAFETY_HEAP, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn direct_recursion_rejected() {
    let (errs, _) = check_src_with_stack("fn f(n: i32) -> i32 { f(n) }\nfn main() -> i32 { f(1) }", None);
    assert!(has_code(&errs, "E_SAFETY_RECURSION"), "expected E_SAFETY_RECURSION, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn indirect_recursion_rejected() {
    let (errs, _) = check_src_with_stack(
        "fn a(n: i32) -> i32 { b(n) }\nfn b(n: i32) -> i32 { a(n) }\nfn main() -> i32 { a(1) }",
        None,
    );
    assert!(has_code(&errs, "E_SAFETY_RECURSION"), "expected E_SAFETY_RECURSION, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn unbounded_loop_rejected() {
    let (errs, _) = check_src_with_stack("fn main() -> i32 { loop { } 0 }", None);
    assert!(has_code(&errs, "E_SAFETY_LOOP"), "expected E_SAFETY_LOOP, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn bounded_loops_pass() {
    let (errs, _) = check_src_with_stack(
        "fn factorial(n: i32) -> i32 { let mut acc = 1; let mut i = n; while i > 1 { acc = acc * i; i = i - 1; } acc }\nfn count() -> i32 { let mut s = 0; for i in 0..10 { s = s + i; } s }\nfn spin() -> i32 { let mut i = 0; loop { i = i + 1; if i > 5 { break; } } i }\nfn main() -> i32 { factorial(5) + count() + spin() }",
        None,
    );
    assert!(errs.is_empty(), "expected zero safety errors, got {:?}", errs.iter().map(|d| format!("{}: {}", d.code.as_deref().unwrap_or("?"), d.message)).collect::<Vec<_>>());
}

#[test]
fn stack_report_and_limit() {
    let src = "fn factorial(n: i32) -> i32 { let mut acc = 1; let mut i = n; while i > 1 { acc = acc * i; i = i - 1; } acc }\nfn main() -> i32 { factorial(5) }";
    let (errs, report) = check_src_with_stack(src, None);
    assert!(errs.is_empty(), "expected zero safety errors without limit, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
    assert!(report.contains("factorial") && report.contains("main"), "stack report should name functions, got: {}", report);
    let (limited, _) = check_src_with_stack(src, Some(1));
    assert!(has_code(&limited, "E_SAFETY_STACK"), "expected E_SAFETY_STACK under limit 1, got {:?}", limited.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn known_profiles_parse() {
    assert!(matches!(parse_profile("AUTOSAR_CPP14"), Some(SafetyProfile::AutosarCpp14)));
    assert!(matches!(parse_profile("DO-178C"), Some(SafetyProfile::Do178C)));
    assert!(matches!(parse_profile("IEC61508"), Some(SafetyProfile::Iec61508)));
    assert!(matches!(parse_profile("none"), Some(SafetyProfile::None)));
    assert!(parse_profile("bogus").is_none());
    let d = unknown_profile_error("bogus");
    assert_eq!(d.code.as_deref(), Some("E_SAFETY_PROFILE"));
}

#[test]
fn user_fn_named_allocate_is_clean() {
    // Regression: substring matching flagged any callee containing "alloc".
    let (errs, _) = check_src_with_stack(
        "fn allocate() -> i32 { 0 }\nfn reallocate(n: i32) -> i32 { n }\nfn main() -> i32 { allocate() + reallocate(1) }",
        None,
    );
    assert!(!has_code(&errs, "E_SAFETY_HEAP"), "user fns must not be heap-flagged, got {:?}", errs.iter().map(|d| format!("{}: {}", d.code.as_deref().unwrap_or("?"), d.message)).collect::<Vec<_>>());
}

#[test]
fn module_kernel_reaches_mir_and_is_skipped() {
    // `module` blocks splice items (no silent swallow); HW-targeted functions
    // are skipped by the CPU profile, not mislabeled as heap.
    let report = check_full(
        "module m {\n    @hw::gpu(backend=cuda, sm=90)\n    kernel fn mm() -> void {\n    }\n    fn main() -> i32 { 0 }\n}",
        None,
    );
    assert!(report.errors.is_empty(), "kernel must be skipped, got {:?}", report.errors.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
    assert_eq!(report.checked_functions, 1, "only main is checked");
    assert_eq!(report.skipped_hw, vec!["mm".to_string()], "mm must be reported skipped");
}

#[test]
fn attr_max_stack_enforced() {
    let src = "#[max_stack(512)]\nfn small() -> i32 { 0 }\nfn main() -> i32 { small() }";
    let (errs, _) = check_src_with_stack(src, None);
    assert!(!has_code(&errs, "E_SAFETY_STACK"), "generous attr must pass, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
    let tight = "#[max_stack(10)]\nfn small() -> i32 { 0 }\nfn main() -> i32 { small() }";
    let (errs2, _) = check_src_with_stack(tight, None);
    assert!(has_code(&errs2, "E_SAFETY_STACK"), "tight attr must fail, got {:?}", errs2.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn attr_wcet_enforced() {
    let src = "#[wcet_cycles(100000)]\nfn small() -> i32 { 0 }\nfn main() -> i32 { small() }";
    let (errs, _) = check_src_with_stack(src, None);
    assert!(errs.is_empty(), "generous wcet must pass, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
    let tight = "#[wcet_cycles(0)]\nfn small() -> i32 { 0 }\nfn main() -> i32 { small() }";
    let (errs2, _) = check_src_with_stack(tight, None);
    assert!(has_code(&errs2, "E_SAFETY_WCET"), "zero wcet bound must fail, got {:?}", errs2.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn tiers_enforced_uniformly() {
    // SafetyTier does not exempt code: unsafe/trusted fns are still checked.
    let (errs, _) = check_src_with_stack("fn unsafe bad() -> i32 { loop { } 0 }\nfn main() -> i32 { 0 }", None);
    assert!(has_code(&errs, "E_SAFETY_LOOP"), "unsafe tier must not exempt loops, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}

#[test]
fn while_true_is_unbounded() {
    // Constant-folded branch: `while true` has no exit edge.
    let (errs, _) = check_src_with_stack("fn main() -> i32 { while true { } 0 }", None);
    assert!(has_code(&errs, "E_SAFETY_LOOP"), "while true must be unbounded, got {:?}", errs.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
    let (ok, _) = check_src_with_stack("fn main() -> i32 { while false { } 0 }", None);
    assert!(!has_code(&ok, "E_SAFETY_LOOP"), "while false must be accepted, got {:?}", ok.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>());
}
