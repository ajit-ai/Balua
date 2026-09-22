//! Object emission for the Balua CPU backend (Phase 2).
//!
//! Two responsibilities:
//!  1. `compile_to_object` — run Cranelift codegen over MIR and emit a native
//!     `.o` relocatable at the requested path.
//!  2. `link_executable` — invoke the host C toolchain (gcc/cc/cl) to link the
//!     object into a runnable executable.

use crate::mir::MirModule;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Compile MIR to a native object file at `out_obj`.
pub fn compile_to_object(modules: &[MirModule], out_obj: &Path, opt_level: u8) -> Result<String> {
    crate::backend::cranelift::compile_modules_to_object(modules, out_obj, opt_level)
}

/// Find a usable C compiler for linking.
/// For cc/gcc/clang we require `--version` to succeed; MSVC `cl` is accepted
/// on successful spawn since it returns non-zero without args.
fn find_linker() -> Option<(&'static str, &'static str)> {
    for (name, arg) in [
        ("cc", "--version"),
        ("gcc", "--version"),
        ("clang", "--version"),
        ("cl", ""),
    ] {
        match Command::new(name).arg(arg).output() {
            Ok(_) if name == "cl" => return Some((name, arg)),
            Ok(out) if out.status.success() => return Some((name, arg)),
            _ => continue,
        }
    }
    None
}

/// Link `obj_path` into an executable at `exe_path` using the host C toolchain.
///
/// Determinism: GNU linkers get `-Wl,--no-insert-timestamp
/// -Wl,--build-id=none` so repeated builds are byte-identical (M3: objects
/// already are; this closes the linked-executable half on gcc/clang/cc).
/// MSVC `cl` has no equivalent probe here (`link /Brepro` is the analogue,
/// unverified on this host) and links as before.
pub fn link_executable(obj_path: &Path, exe_path: &Path) -> Result<()> {
    let Some((linker, _)) = find_linker() else {
        return Err(anyhow!(
            "E_UNSUPPORTED_ON_TARGET: no system C linker (cc/gcc/clang) found on PATH"
        ));
    };
    let mut cmd = Command::new(linker);
    if linker != "cl" {
        cmd.arg("-Wl,--no-insert-timestamp").arg("-Wl,--build-id=none");
    }
    cmd.arg(obj_path).arg("-o").arg(exe_path);
    // Windows executable extension if not provided
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn linker `{}`", linker))?;
    if !output.status.success() {
        return Err(anyhow!(
            "linker `{}` failed: {}",
            linker,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Full pipeline: compile MIR -> object -> executable.
/// Returns the path of the produced executable.
pub fn build_executable(
    modules: &[MirModule],
    out_exe: &Path,
    opt_level: u8,
    keep_object: bool,
) -> Result<PathBuf> {
    let mut obj_path = out_exe.with_extension("o").to_path_buf();
    let _ = &mut obj_path;
    // Use a temp object next to the exe.
    let obj = temp_object_path(out_exe);
    compile_to_object(modules, &obj, opt_level)?;
    let exe = ensure_exe_extension(out_exe);
    link_executable(&obj, &exe)?;
    if !keep_object {
        let _ = std::fs::remove_file(&obj);
    }
    Ok(exe)
}

fn temp_object_path(out_exe: &Path) -> PathBuf {
    let mut p = out_exe.as_os_str().to_owned();
    p.push(".o");
    PathBuf::from(p)
}

pub(crate) fn ensure_exe_extension(out: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if out.extension().is_none() {
            let mut os = out.as_os_str().to_owned();
            os.push(".exe");
            return PathBuf::from(os);
        }
    }
    out.to_path_buf()
}
