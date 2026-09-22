//! Phase 2 execution tests — end-to-end: compile a Balua source file with the
//! Cranelift CPU backend, link it into a native executable, run it, and assert
//! the process exit code.
//!
//! These prove the backend actually *runs* programs (not just "compiles" them).
//! Each case is a small program whose exit code is the value of `main`'s return.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Locate the compiled `balua` driver binary (provided by cargo for
/// integration tests via CARGO_BIN_EXE_<name>).
fn balua_bin() -> PathBuf {
    let p = env!("CARGO_BIN_EXE_balua");
    PathBuf::from(p)
}

/// A single executable test case: a Balua source snippet and the expected
/// process exit code after compile + link + run.
struct Case {
    name: &'static str,
    src: &'static str,
    expected: i32,
}

const CASES: &[Case] = &[
    Case {
        name: "return_literal",
        src: "fn main() -> i32 { 7 }",
        expected: 7,
    },
    Case {
        name: "arithmetic",
        src: "fn main() -> i32 { let x = 6; let y = 7; x * y }",
        expected: 42,
    },
    Case {
        name: "while_loop",
        src: "fn main() -> i32 { let mut i = 0; let mut sum = 0; while i < 5 { sum = sum + 1; i = i + 1; } sum }",
        expected: 5,
    },
    Case {
        name: "loop_break",
        src: "fn main() -> i32 { let mut i = 0; let mut acc = 0; loop { i = i + 1; if i > 5 { break; } acc = acc + 1; } acc }",
        expected: 5,
    },
    Case {
        name: "while_continue",
        src: "fn main() -> i32 { let mut i = 0; let mut acc = 0; while i < 5 { i = i + 1; if i == 3 { continue; } acc = acc + 1; } acc }",
        expected: 4,
    },
    Case {
        name: "factorial",
        src: "fn main() -> i32 {\n    let mut n = 5;\n    let mut acc = 1;\n    while n > 1 {\n        acc = acc * n;\n        n = n - 1;\n    }\n    acc\n}",
        expected: 120,
    },
    Case {
        name: "for_range",
        src: "fn main() -> i32 { let mut s = 0; for i in 0..5 { s = s + i; } s }",
        expected: 10,
    },
    Case {
        name: "for_break",
        src: "fn main() -> i32 { let mut s = 0; for i in 0..10 { if i >= 4 { break; } s = s + i; } s }",
        expected: 6,
    },
    Case {
        name: "nested_else_if",
        src: "fn main() -> i32 { let x = 7; let mut r = 0; if x < 5 { r = 1; } else { if x < 10 { r = 2; } else { r = 3; } } r }",
        expected: 2,
    },
    Case {
        name: "func_call_param",
        src: "fn double(n: i32) -> i32 { n * 2 }\nfn main() -> i32 { double(21) }",
        expected: 42,
    },
    Case {
        name: "conditional",
        src: "fn main() -> i32 { let x = 3; if x > 0 { 1 } else { 0 } }",
        expected: 1,
    },
    Case {
        name: "match_expr",
        src: "fn main() -> i32 { let x = 2; let r = match x { 1 => 10, 2 => 20, _ => 0 }; r }",
        expected: 20,
    },
    Case {
        name: "match_default",
        src: "fn main() -> i32 { let x = 9; let r = match x { 1 => 10, 2 => 20, _ => 0 }; r }",
        expected: 0,
    },
    Case {
        name: "multi_param_call",
        src: "fn add(a: i32, b: i32) -> i32 { a + b }\nfn main() -> i32 { add(20, 22) }",
        expected: 42,
    },
    Case {
        name: "nested_loop",
        src: "fn main() -> i32 { let mut s = 0; for i in 0..3 { let mut j = 0; while j < 2 { s = s + 1; j = j + 1; } } s }",
        expected: 6,
    },
    Case {
        name: "early_return",
        src: "fn foo() -> i32 { return 5; 6 }\nfn main() -> i32 { foo() }",
        expected: 5,
    },
    Case {
        name: "bitwise",
        src: "fn main() -> i32 { let a = 5; let b = 3; let c = a & b; let d = a | b; let e = a ^ b; c + d + e }",
        expected: 14,
    },
];

/// Compile + link + run a Balua source, returning its exit code.
/// Fails the test if compilation, linking, or the run times out (infinite loop).
fn run_case(name: &str, src: &str) -> Result<i32, String> {
    let dir = std::env::temp_dir().join(format!("balua_exec_{}_{}", name, std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {}", e))?;
    let src_path = dir.join("prog.bl");
    let exe_path = dir.join("prog.exe");
    std::fs::write(&src_path, src).map_err(|e| format!("write src: {}", e))?;

    // Compile + link (balua driver, -o produces a linked executable).
    // Backend under test comes from BALUA_CPU_BACKEND (default cranelift);
    // the llvm CI job sets it to llvm with `--features llvm`, so this one
    // suite exercises both CPU backends.
    let backend = std::env::var("BALUA_CPU_BACKEND").unwrap_or_else(|_| "cranelift".into());
    let status = Command::new(balua_bin())
        .arg(&src_path)
        .arg("--cpu-backend")
        .arg(&backend)
        .arg("-o")
        .arg(&exe_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| format!("spawn balua: {}", e))?;
    if !status.success() {
        return Err(format!("compile/link failed (exit {})", status.code().unwrap_or(-1)));
    }
    if !exe_path.exists() {
        return Err("no executable produced".into());
    }

    // Run the produced executable with a hard timeout to catch infinite loops.
    let mut child = Command::new(&exe_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn exe: {}", e))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().map_err(|e| format!("wait: {}", e))? {
            return Ok(status.code().unwrap_or(-1));
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            return Err("exe timed out (infinite loop?)".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn exit_criteria_execute() {
    let mut failures = 0;
    for case in CASES {
        match run_case(case.name, case.src) {
            Ok(code) if code == case.expected => {
                println!("[ok] {} -> {}", case.name, code);
            }
            Ok(code) => {
                failures += 1;
                eprintln!("[FAIL] {} expected {} got {}", case.name, case.expected, code);
            }
            Err(e) => {
                failures += 1;
                eprintln!("[FAIL] {} : {}", case.name, e);
            }
        }
    }
    assert!(failures == 0, "{} execution case(s) failed", failures);
}

/// Sanity: produce a runnable executable at all.
#[test]
fn produces_linked_executable() {
    let dir = std::env::temp_dir().join(format!("balua_exec_link_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src_path = dir.join("prog.bl");
    let exe_path = dir.join("prog.exe");
    std::fs::write(&src_path, "fn main() -> i32 { 1 }").unwrap();

    let out = Command::new(balua_bin())
        .arg(&src_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run balua");
    assert!(out.status.success(), "balua failed: {}", String::from_utf8_lossy(&out.stderr));
    assert!(exe_path.is_file(), "no executable produced");
}

/// Ensure the produced path helper type-checks away unused-import warnings.
#[allow(dead_code)]
fn _keep_path_import() -> &'static Path {
    Path::new(".")
}
