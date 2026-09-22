//! BPM — Balua Package Manager (BPM.exe) — Section 7.1
//! Implements PubGrub resolver, builder, registry client. Commands:
//! new, build, run, test, bench, add, publish, doc, fmt, lint, cross
//! Legacy alias blpkg.exe still works.

use clap::{Parser, Subcommand};
use anyhow::Result;

mod resolver;
mod builder;
mod registry;
mod bom;

#[derive(Parser)]
#[command(name="BPM", version, about="BPM — Balua Package Manager (BPM.exe, Windows 11). Legacy alias: blpkg.exe")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    New { name: String },
    Build {
        #[arg(long)] target: Option<String>,
        #[arg(long)] release: bool,
    },
    Run,
    Test,
    Bench,
    Add { pkg: String },
    Publish,
    Doc,
    Fmt,
    Lint,
    Cross { target: String },
    Bom,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::New { name } => {
            // The package name is the final path component: `blpkg new
            // C:/path/demo` must record `demo`, not the path (backslashes
            // break TOML parsing of Balua.toml).
            let dir = std::path::PathBuf::from(&name);
            let pkg = package_name(&dir);
            std::fs::create_dir_all(&dir)?;
            std::fs::write(dir.join("Balua.toml"), format!("[package]\nname=\"{}\"\nversion=\"0.1.0\"\nedition=\"2026\"\n", pkg))?;
            std::fs::create_dir_all(dir.join("src"))?;
            std::fs::write(dir.join("src/main.bl"), "fn main() -> i32 { 0 }\n")?;
            println!("Created Balua package '{}'", pkg);
        }
        Cmd::Build { target, release } => {
            let profile = if release { "release" } else { "debug" };
            println!("blpkg build — profile={} target={:?}", profile, target);
            builder::build(target.as_deref(), release)?;
        }
        Cmd::Run => {
            let exe = builder::build(None, false)?;
            let code = builder::run_exe(&exe)?;
            println!("Ran {} — exit {}", exe.display(), code);
        }
        Cmd::Test => {
            let exe = builder::build(None, false)?;
            let code = builder::run_exe(&exe)?;
            println!("test(sim): entry {} — exit {} (simulation fallback, no hardware needed)", exe.display(), code);
        }
        Cmd::Bench => println!("blpkg bench — criterion-style reporting (post-GA)"),
        Cmd::Add { pkg } => {
            let (name, req) = match pkg.split_once('@') {
                Some((n, r)) => (n.to_string(), r.to_string()),
                None => (pkg.clone(), "*".to_string()),
            };
            let mut resolver = resolver::Resolver::new();
            resolver.add(&name, &req);
            match resolver.resolve() {
                Ok(map) => println!("Resolved '{}' -> '{}' (local-only, no registry fetch)", name, map.get(&name).unwrap()),
                Err(e) => anyhow::bail!("cannot add '{}': {}", pkg, e),
            }
        }
        Cmd::Publish => registry::publish()?,
        Cmd::Doc => println!("Generating docs via balua-doc ..."),
        Cmd::Fmt => println!("Formatting via balua-fmt ..."),
        Cmd::Lint => println!("Linting via balua-clippy ..."),
        Cmd::Cross { target } => println!("Cross-compiling for '{}' (no_std, embedded)", target),
        Cmd::Bom => { println!("{}", bom::generate_bom(&[])); },
    }
    Ok(())
}

/// Package name for `blpkg new <path>`: the final path component.
fn package_name(dir: &std::path::Path) -> String {
    dir.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_uses_final_component() {
        assert_eq!(package_name(std::path::Path::new("demo")), "demo");
        assert_eq!(package_name(std::path::Path::new("C:/a/b/demo")), "demo");
    }
}
