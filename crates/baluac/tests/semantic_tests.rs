//! Phase 1 exit-criteria tests for semantic analysis.
//! Proves that InferenceEngine and BorrowChecker are wired in and producing real diagnostics.

use baluac_lib::lexer::Lexer;
use baluac_lib::parser::Parser;
use baluac_lib::semantic::SemanticAnalyzer;

/// Helper: lex → parse → semantic-analyze a source string.
/// Returns (diagnostics, type_table).
fn analyze_src(src: &str) -> (Vec<baluac_lib::diagnostics::Diagnostic>, baluac_lib::semantic::TypeTable) {
    let mut lexer = Lexer::new(src, "test.bl");
    let (tokens, lex_diags) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (program, parse_diags) = parser.parse_program();
    let mut analyzer = SemanticAnalyzer::new();
    let (mut sem_diags, type_table) = analyzer.analyze(&program);
    // Prepend lex/parse diagnostics so the caller sees everything
    let mut all = lex_diags;
    all.extend(parse_diags);
    all.extend(sem_diags);
    sem_diags = all;
    (sem_diags, type_table)
}

fn has_code(diags: &[baluac_lib::diagnostics::Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code.as_deref() == Some(code) && matches!(d.severity, baluac_lib::diagnostics::Severity::Error))
}

fn has_no_errors(diags: &[baluac_lib::diagnostics::Diagnostic]) -> bool {
    !diags.iter().any(|d| matches!(d.severity, baluac_lib::diagnostics::Severity::Error))
}

// ── Exit criterion 1: type mismatch ────────────────────────────────────

#[test]
fn type_mismatch_let_init_rejects() {
    // Assigning a bool literal to an i32 let binding → E_TYPE_MISMATCH
    let src = r#"
        fn main() -> i32 {
            let x: i32 = true;
            0
        }
    "#;
    let (diags, _) = analyze_src(src);
    assert!(
        has_code(&diags, "E_TYPE_MISMATCH"),
        "Expected E_TYPE_MISMATCH for `let x: i32 = true`, got: {:?}",
        diags.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}

#[test]
fn type_mismatch_binary_op_rejects() {
    // Adding i32 + bool → E_TYPE_MISMATCH (operand types differ)
    let src = r#"
        fn main() -> i32 {
            let a = 1;
            let b = true;
            let c = a + b;
            0
        }
    "#;
    let (diags, _) = analyze_src(src);
    assert!(
        has_code(&diags, "E_TYPE_MISMATCH"),
        "Expected E_TYPE_MISMATCH for `i32 + bool`, got: {:?}",
        diags.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}

// ── Exit criterion 2: use-after-move ───────────────────────────────────

#[test]
fn use_after_move_device_rejects() {
    // Move a variable to device, then try to use it on host
    let src = r#"
        fn main() -> i32 {
            let data = 42;
            let gpu_data = move_to_device(data, "gpu");
            let x = data;
            0
        }
    "#;
    let (diags, _) = analyze_src(src);
    assert!(
        has_code(&diags, "E_USE_AFTER_MOVE"),
        "Expected E_USE_AFTER_MOVE after move_to_device, got: {:?}",
        diags.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}

// ── Exit criterion 3: borrow conflict ──────────────────────────────────

#[test]
fn borrow_conflict_mut_while_shared_rejects() {
    // Take a mutable borrow while a shared borrow is alive → E_BORROW_CONFLICT
    let src = r#"
        fn main() -> i32 {
            let data = 42;
            let shared_ref = &data;
            let mut_ref = &mut data;
            0
        }
    "#;
    let (diags, _) = analyze_src(src);
    assert!(
        has_code(&diags, "E_BORROW_CONFLICT"),
        "Expected E_BORROW_CONFLICT for mut borrow while shared alive, got: {:?}",
        diags.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}

// ── Exit criterion 4: valid programs produce zero errors ────────────────

#[test]
fn valid_hello_program_has_no_errors() {
    let src = r#"
        fn main() -> i32 {
            let x = 42;
            x
        }
    "#;
    let (diags, _) = analyze_src(src);
    assert!(
        has_no_errors(&diags),
        "Expected zero errors for valid program, got: {:?}",
        diags.iter().map(|d| format!("{}: {}", d.code.as_deref().unwrap_or("?"), d.message)).collect::<Vec<_>>()
    );
}

#[test]
fn valid_factorial_has_no_errors() {
    let src = r#"
        fn factorial(n: i32) -> i32 {
            let mut result = 1;
            let mut i = n;
            while i > 1 {
                result = result * i;
                i = i - 1;
            }
            result
        }
        fn main() -> i32 {
            let x = factorial(5);
            x
        }
    "#;
    let (diags, _) = analyze_src(src);
    assert!(
        has_no_errors(&diags),
        "Expected zero errors for factorial, got: {:?}",
        diags.iter().map(|d| format!("{}: {}", d.code.as_deref().unwrap_or("?"), d.message)).collect::<Vec<_>>()
    );
}

#[test]
fn valid_if_else_has_no_errors() {
    let src = r#"
        fn main() -> i32 {
            let x = 10;
            if x > 5 { 1 } else { 0 }
        }
    "#;
    let (diags, _) = analyze_src(src);
    assert!(
        has_no_errors(&diags),
        "Expected zero errors for valid if/else, got: {:?}",
        diags.iter().map(|d| format!("{}: {}", d.code.as_deref().unwrap_or("?"), d.message)).collect::<Vec<_>>()
    );
}

// ── Bonus: type table is populated ─────────────────────────────────────

#[test]
fn type_table_populated_for_literals() {
    let src = r#"
        fn main() -> i32 {
            let x = 42;
            x
        }
    "#;
    let (diags, type_table) = analyze_src(src);
    assert!(has_no_errors(&diags));
    // TypeTable should have at least one entry (the literal 42)
    assert!(
        !type_table.is_empty(),
        "TypeTable should be populated for a valid program"
    );
}
