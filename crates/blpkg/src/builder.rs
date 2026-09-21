//! blpkg builder — invokes baluac as a library per Balua.toml (M1 local-only).
//!
//! Entry convention: `src/main.bl`, falling back to the first `src/*.bl`.
//! Output: `target/debug/<name>[.exe]` (debug) or `target/release/<name>[.exe]`.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Minimal manifest view for local builds.
#[derive(Debug)]
pub struct Manifest {
    pub name: String,
    pub entry: PathBuf,
}

/// Read `Balua.toml` in `dir` and resolve the entry file.
pub fn read_manifest(dir: &Path) -> Result<Manifest> {
    let text = std::fs::read_to_string(dir.join("Balua.toml")).context("missing Balua.toml (run `blpkg new`)")?;
    let value: toml::Value = text.parse().context("invalid Balua.toml")?;
    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("app")
        .to_string();
    let entry = dir.join("src/main.bl");
    let entry = if entry.exists() {
        entry
    } else {
        let mut found: Option<PathBuf> = None;
        if let Ok(rd) = std::fs::read_dir(dir.join("src")) {
            for e in rd.flatten() {
                if e.path().extension().and_then(|x| x.to_str()) == Some("bl") {
                    found = Some(e.path());
                    break;
                }
            }
        }
        found.context("no .bl entry under src/")?
    };
    Ok(Manifest { name, entry })
}

/// Warn (don't fail) on non-CPU targets: heterogeneous execution is post-GA.
pub fn check_target(target: Option<&str>) {
    match target {
        None => {}
        Some(t) if t.starts_with("gpu:cuda") => eprintln!("note: Backend PTX (SM 70-90) is post-GA — building CPU entry only"),
        Some(t) if t.starts_with("fpga:") => eprintln!("note: Backend HLS C / Vitis is post-GA — building CPU entry only"),
        Some(t) if t.starts_with("quantum:") => eprintln!("note: Backend OpenQASM 3.0 is post-GA — building CPU entry only"),
        Some(t) if t.starts_with("wasm") => eprintln!("note: Backend WASM is post-GA — building CPU entry only"),
        Some(t) => eprintln!("note: Unknown target '{}' — building CPU entry only", t),
    }
}

/// Compile one Balua source file to a native executable via the baluac library.
/// Fails on any `Error` diagnostic instead of linking invalid code.
pub fn compile_source(src: &Path, out_exe: &Path, release: bool) -> Result<PathBuf> {
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
        bail!("{}:{}:{}: error: {}", src.display(), first.span.as_ref().map(|s| s.line).unwrap_or(0), first.span.as_ref().map(|s| s.col).unwrap_or(0), first.message);
    }
    let mir = baluac_lib::mir::MirBuilder::lower_with_types(&program, &type_table);
    let opt: u8 = if release { 2 } else { 0 };
    let exe = baluac_lib::backend::object_emit::build_executable(&mir, out_exe, opt, false)?;
    Ok(exe)
}

fn out_path(name: &str, release: bool) -> PathBuf {
    let dir = if release { "target/release" } else { "target/debug" };
    let mut p = PathBuf::from(dir);
    p.push(name);
    #[cfg(target_os = "windows")]
    p.set_extension("exe");
    p
}

/// Build the package in the current directory. Returns the artifact path.
pub fn build(target: Option<&str>, release: bool) -> Result<PathBuf> {
    check_target(target);
    let cwd = std::env::current_dir()?;
    let manifest = read_manifest(&cwd)?;
    if release {
        println!("Profile: release (opt=2)");
    }
    let out = out_path(&manifest.name, release);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let exe = compile_source(&manifest.entry, &out, release)?;
    println!("Build succeeded: {} -> {}", manifest.entry.display(), exe.display());
    Ok(exe)
}

/// Execute a produced artifact and return its exit code.
pub fn run_exe(exe: &Path) -> Result<i32> {
    let status = std::process::Command::new(exe).status().with_context(|| format!("run {}", exe.display()))?;
    Ok(status.code().unwrap_or(-1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scaffold(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("blpkg_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("Balua.toml"), format!("[package]\nname=\"{}\"\nversion=\"0.1.0\"\nedition=\"2026\"\n", name)).unwrap();
        std::fs::write(dir.join("src/main.bl"), "fn main() -> i32 { 0 }\n").unwrap();
        dir
    }

    #[test]
    fn manifest_resolves_main_entry() {
        let dir = scaffold("manifest_main");
        let m = read_manifest(&dir).unwrap();
        assert_eq!(m.name, "manifest_main");
        assert!(m.entry.ends_with("src/main.bl"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_falls_back_to_first_bl() {
        let dir = scaffold("manifest_fallback");
        std::fs::remove_file(dir.join("src/main.bl")).unwrap();
        std::fs::write(dir.join("src/other.bl"), "fn main() -> i32 { 0 }\n").unwrap();
        let m = read_manifest(&dir).unwrap();
        assert!(m.entry.ends_with("src/other.bl"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_missing_is_error() {
        let dir = std::env::temp_dir().join(format!("blpkg_missing_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read_manifest(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
