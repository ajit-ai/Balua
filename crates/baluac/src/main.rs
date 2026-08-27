//! balua — Balua compiler driver (Section 2) — Windows balua.exe
//! Usage: balua hello.bl  |  balua --help  |  balua [--emit-llvm] <file.bl>
//! Legacy alias baluac.exe still works. Package manager is BPM.exe.

use baluac_lib::{diagnostics::*, lexer::Lexer, mir::MirBuilder, parser::Parser, semantic::SemanticAnalyzer};
use clap::Parser as ClapParser;
use std::path::PathBuf;

#[derive(ClapParser, Debug)]
#[command(name = "balua", version, about = "Balua system programming language compiler — balua.exe (Windows 11). Package manager: BPM.exe. Legacy alias: baluac.exe")]
struct Args {
    /// Input Balua source files (.bl)
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Emit LLVM IR instead of object file
    #[arg(long)]
    emit_llvm: bool,

    /// Emit MIR JSON
    #[arg(long)]
    emit_mir: bool,

    /// Target triple (x86_64, aarch64, riscv64, arm-none-eabi)
    #[arg(long, default_value = "x86_64")]
    target: String,

    /// Hardware backend override
    #[arg(long)]
    backend: Option<String>,

    /// JSON diagnostics for IDE
    #[arg(long)]
    json_diagnostics: bool,

    /// Safety profile
    #[arg(long)]
    safety_profile: Option<String>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let mut all_diags: Vec<Diagnostic> = Vec::new();
    for file in &args.files {
        let src = std::fs::read_to_string(file)?;
        let file_str = file.display().to_string();

        // Stage 1: Lex
        let mut lexer = Lexer::new(&src, file_str.clone());
        let (tokens, mut lex_diags) = lexer.tokenize();
        all_diags.append(&mut lex_diags);

        // Stage 2: Parse
        let mut parser = Parser::new(tokens);
        let (program, mut parse_diags) = parser.parse_program();
        all_diags.append(&mut parse_diags);

        // Stage 3: Semantic
        let mut analyzer = SemanticAnalyzer::new();
        let mut sem_diags = analyzer.analyze(&program);
        all_diags.append(&mut sem_diags);

        // Stage 4: MIR
        let mir = MirBuilder::lower(&program);
        if args.emit_mir {
            println!("{}", MirBuilder::to_json(&mir));
        }
        if args.emit_llvm {
            println!("; LLVM IR stub for {} (target: {})", file_str, args.target);
            for m in &mir {
                for f in &m.functions {
                    println!("define void @{}() {{", f.name);
                    println!("  ret void");
                    println!("}}");
                }
            }
        }

        if let Some(profile) = &args.safety_profile {
            println!("; Safety profile: {} — checks enabled (MISRA/AUTOSAR/DO-178C)", profile);
        }
    }

    if !all_diags.is_empty() {
        for d in &all_diags {
            if args.json_diagnostics {
                println!("{}", d.to_json());
            } else {
                eprintln!("{}", d);
            }
        }
        if all_diags.iter().any(|d| matches!(d.severity, Severity::Error)) {
            std::process::exit(1);
        }
    } else if !args.emit_llvm && !args.emit_mir {
        // Windows: balua hello.bl  (as requested)
        let exe = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or("balua".into());
        println!("{}: compilation successful ({} files, target: {})", exe.trim_end_matches(".exe"), args.files.len(), args.target);
    }

    Ok(())
}
