//! CLI freeze tests — the `--help` surface is a compatibility contract.
//! If a flag is renamed or removed, these fail on purpose: update the test,
//! the docs, and the changelog together.

use std::path::PathBuf;
use std::process::Command;

fn balua_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_balua"))
}

fn help_text() -> String {
    let out = Command::new(balua_bin())
        .arg("--help")
        .output()
        .expect("run balua --help");
    assert!(out.status.success(), "balua --help failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Flags that must keep working ( M3 CLI freeze).
#[test]
fn cli_flags_stable() {
    let help = help_text();
    for flag in [
        "--emit-llvm",
        "--emit-mir",
        "--emit-clif",
        "--target",
        "--backend",
        "--cpu-backend",
        "--release",
        "--keep-object",
        "--json-diagnostics",
        "--safety-profile",
        "--safety-stack-limit",
        "--verbose",
        "-o",
    ] {
        assert!(help.contains(flag), "CLI flag {:?} missing from --help", flag);
    }
}

/// Release version stamp (M4): `balua --version` reports the GA release.
#[test]
fn cli_version_is_ga() {
    let out = Command::new(balua_bin())
        .arg("--version")
        .output()
        .expect("run balua --version");
    assert!(out.status.success(), "balua --version failed");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(text.contains("v1.0-ga"), "expected GA version stamp, got {:?}", text);
}

/// Driver stays a multi-file compiler: passing two inputs is accepted.
#[test]
fn cli_accepts_multiple_files() {
    let dir = std::env::temp_dir().join(format!("balua_cli_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("a.bl");
    let b = dir.join("b.bl");
    std::fs::write(&a, "fn main() -> i32 { 0 }\n").unwrap();
    std::fs::write(&b, "fn main() -> i32 { 0 }\n").unwrap();
    let out = Command::new(balua_bin())
        .arg(&a)
        .arg(&b)
        .arg("--emit-mir")
        .output()
        .expect("run balua");
    assert!(out.status.success(), "multi-file --emit-mir failed: {}", String::from_utf8_lossy(&out.stderr));
}
