//! balua-test — Balua test runner (M3).
//!
//! Convention over configuration: each target file is a Balua program whose
//! `main` returns an exit code. A target passes iff it compiles, links, and
//! exits 0 — the same convention the repo's self-checking examples use
//! (`... if ok { 0 } else { 1 }`). `#[bench]` is recognized and reported as
//! timing (mean of repeated runs); there is no Balua-level unit-test
//! attribute yet (post-GA scope).
//!
//! Usage: `balua-test [--bench[=N]] <files-or-dirs...>` (dirs searched for
//! `*.bl`, non-recursively). Exit code is the number of failures.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct CaseResult {
    name: String,
    ok: bool,
    detail: String,
}

/// Compile one Balua source to a native executable via the baluac library.
fn compile(src: &Path, out_exe: &Path, release: bool) -> Result<()> {
    let text = std::fs::read_to_string(src).with_context(|| format!("read {}", src.display()))?;
    let file_str = src.display().to_string();
    let mut lexer = baluac_lib::lexer::Lexer::new(&text, file_str);
    let (tokens, lex_diags) = lexer.tokenize();
    let mut parser = baluac_lib::parser::Parser::new(tokens);
    let (program, parse_diags) = parser.parse_program();
    let mut analyzer = baluac_lib::semantic::SemanticAnalyzer::new();
    let (sem_diags, type_table) = analyzer.analyze(&program);
    let mut all = lex_diags;
    all.extend(parse_diags);
    all.extend(sem_diags);
    if let Some(first) = all.iter().find(|d| matches!(d.severity, baluac_lib::diagnostics::Severity::Error)) {
        bail!("{}: error: {}", src.display(), first.message);
    }
    let mir = baluac_lib::mir::MirBuilder::lower_with_types(&program, &type_table);
    let opt: u8 = if release { 2 } else { 0 };
    baluac_lib::backend::object_emit::build_executable(&mir, out_exe, opt, false)?;
    Ok(())
}

/// Run an executable with a hard timeout; returns (exit_code, elapsed).
fn run_with_timeout(exe: &Path, timeout: Duration) -> Result<(i32, Duration)> {
    let start = Instant::now();
    let mut child = Command::new(exe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn {}", exe.display()))?;
    loop {
        if let Some(status) = child.try_wait().context("wait for test exe")? {
            return Ok((status.code().unwrap_or(-1), start.elapsed()));
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            bail!("timed out after {:?}", timeout);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn run_case(src: &Path, bench_iters: Option<usize>) -> CaseResult {
    let name = src.display().to_string();
    let dir = std::env::temp_dir().join(format!("balua_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("prog");
    let exe = dir.join(format!("{}.exe", stem));
    if let Err(e) = compile(src, &exe, false) {
        return CaseResult { name, ok: false, detail: format!("compile: {:#}", e) };
    }
    let iters = bench_iters.unwrap_or(1);
    let mut codes = Vec::new();
    let mut total = Duration::ZERO;
    for _ in 0..iters {
        match run_with_timeout(&exe, Duration::from_secs(10)) {
            Ok((code, elapsed)) => {
                codes.push(code);
                total += elapsed;
            }
            Err(e) => {
                return CaseResult { name, ok: false, detail: format!("run: {:#}", e) };
            }
        }
    }
    let _ = std::fs::remove_file(&exe);
    if codes.iter().any(|&c| c != 0) {
        return CaseResult { name, ok: false, detail: format!("exit codes {:?}", codes) };
    }
    if bench_iters.is_some() {
        CaseResult { name, ok: true, detail: format!("exit 0 x{}; mean {:?}", iters, total / iters as u32) }
    } else {
        CaseResult { name, ok: true, detail: "exit 0".into() }
    }
}

fn collect_targets(args: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for arg in args {
        let p = PathBuf::from(arg);
        if p.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&p) {
                let mut names: Vec<PathBuf> = rd
                    .flatten()
                    .map(|e| e.path())
                    .filter(|q| q.extension().and_then(|x| x.to_str()) == Some("bl"))
                    .collect();
                names.sort();
                out.extend(names);
            }
        } else {
            out.push(p);
        }
    }
    out
}

fn print_usage() {
    eprintln!("usage: balua-test [--bench[=N]] <files-or-dirs...>");
    eprintln!("  pass convention: program compiles, links, and exits 0");
    eprintln!("  --bench[=N]: run each target N times (default 5), report mean time");
}

fn main() {
    let mut bench: Option<usize> = None;
    let mut args: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        if arg == "--bench" {
            bench = Some(5);
        } else if let Some(n) = arg.strip_prefix("--bench=") {
            match n.parse::<usize>() {
                Ok(v) if v > 0 => bench = Some(v),
                _ => {
                    eprintln!("balua-test: invalid --bench value {:?}", n);
                    std::process::exit(2);
                }
            }
        } else if arg == "-h" || arg == "--help" {
            print_usage();
            return;
        } else {
            args.push(arg);
        }
    }
    if args.is_empty() {
        print_usage();
        std::process::exit(2);
    }
    let targets = collect_targets(&args);
    if targets.is_empty() {
        eprintln!("balua-test: no .bl targets found");
        std::process::exit(2);
    }
    let mut failed = 0;
    for target in &targets {
        let r = run_case(target, bench);
        if r.ok {
            println!("[ok] {} — {}", r.name, r.detail);
        } else {
            failed += 1;
            eprintln!("[FAIL] {} — {}", r.name, r.detail);
        }
    }
    println!("{} passed, {} failed ({} targets)", targets.len() - failed, failed, targets.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_case(name: &str, src: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("balua_test_ut_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, src).unwrap();
        p
    }

    #[test]
    fn passing_program_reports_ok() {
        let p = write_case("pass.bl", "fn main() -> i32 { 0 }");
        let r = run_case(&p, None);
        assert!(r.ok, "expected ok, got {}", r.detail);
    }

    #[test]
    fn nonzero_exit_reports_failure() {
        let p = write_case("fail.bl", "fn main() -> i32 { 3 }");
        let r = run_case(&p, None);
        assert!(!r.ok, "expected failure for exit 3");
        assert!(r.detail.contains("[3]") || r.detail.contains('3'));
    }

    #[test]
    fn compile_error_reports_failure() {
        let p = write_case("bad.bl", "fn main() -> i32 { let x: i32 = true; x }");
        let r = run_case(&p, None);
        assert!(!r.ok, "expected compile failure");
        assert!(r.detail.starts_with("compile:"));
    }

    #[test]
    fn bench_reports_mean() {
        let p = write_case("bench.bl", "fn main() -> i32 { 0 }");
        let r = run_case(&p, Some(2));
        assert!(r.ok, "expected ok, got {}", r.detail);
        assert!(r.detail.contains("mean"), "expected bench timing, got {}", r.detail);
    }
}
