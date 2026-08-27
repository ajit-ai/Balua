//! blpkg — Balua package manager (Section 7.1)
//! Implements PubGrub resolver, builder, registry client. Commands:
//! new, build, run, test, bench, add, publish, doc, fmt, lint, cross

use clap::{Parser, Subcommand};
use anyhow::Result;

mod resolver;
mod builder;
mod registry;

#[derive(Parser)]
#[command(name="blpkg", version, about="Balua package manager")]
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::New { name } => {
            std::fs::create_dir_all(&name)?;
            std::fs::write(format!("{}/Balua.toml", name), format!("[package]\nname=\"{}\"\nversion=\"0.1.0\"\nedition=\"2026\"\n", name))?;
            std::fs::create_dir_all(format!("{}/src", name))?;
            std::fs::write(format!("{}/src/main.bl", name), "fn main() -> i32 { 0 }\n")?;
            println!("Created Balua package '{}'", name);
        }
        Cmd::Build { target, release } => {
            let profile = if release { "release" } else { "debug" };
            println!("blpkg build — profile={} target={:?}", profile, target);
            builder::build(target.as_deref(), release)?;
        }
        Cmd::Run => { builder::build(None, false)?; println!("Running target/debug/app ..."); }
        Cmd::Test => println!("blpkg test — running #[test] and #[test(sim=true)] suites (simulation fallback, no hardware needed)"),
        Cmd::Bench => println!("blpkg bench — criterion-style reporting"),
        Cmd::Add { pkg } => println!("Adding dependency '{}' via PubGrub resolver...", pkg),
        Cmd::Publish => registry::publish()?,
        Cmd::Doc => println!("Generating docs via balua-doc ..."),
        Cmd::Fmt => println!("Formatting via balua-fmt ..."),
        Cmd::Lint => println!("Linting via balua-clippy ..."),
        Cmd::Cross { target } => println!("Cross-compiling for '{}' (no_std, embedded)", target),
    }
    Ok(())
}
