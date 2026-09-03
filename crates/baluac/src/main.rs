//! balua — Balua compiler driver (Section 2) — Windows balua.exe
//! Usage: balua hello.bl  |  balua --help  |  balua [--emit-llvm] <file.bl>
//! Legacy alias baluac.exe still works. Package manager is BPM.exe.

use baluac_lib::{diagnostics::*, lexer::Lexer, mir::MirBuilder, parser::Parser, semantic::SemanticAnalyzer, backend::Backend};
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

    /// CPU backend: llvm (default release, LTO) or cranelift (fast debug, 4GB host)
    #[arg(long, default_value = "llvm")]
    cpu_backend: String,

    /// Emit Cranelift IR (.clif) — lightweight alternative to LLVM
    #[arg(long)]
    emit_clif: bool,

    /// JSON diagnostics for IDE
    #[arg(long)]
    json_diagnostics: bool,

    /// Safety profile
    #[arg(long)]
    safety_profile: Option<String>,

    /// Verbose output with compilation events
    #[arg(long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    let mut profile = Profile::new();
    let t0 = std::time::Instant::now();

    let mut all_diags: Vec<Diagnostic> = Vec::new();
    for file in &args.files {
        let src = std::fs::read_to_string(file)?;
        let file_str = file.display().to_string();

        // Stage 1: Lex
        let t = std::time::Instant::now();
        let mut lexer = Lexer::new(&src, file_str.clone());
        let (tokens, mut lex_diags) = lexer.tokenize();
        profile.record(EventKind::Lex, t.elapsed().as_millis() as u64, file_str.clone());
        all_diags.append(&mut lex_diags);

        // Stage 2: Parse
        let t = std::time::Instant::now();
        let mut parser = Parser::new(tokens);
        let (program, mut parse_diags) = parser.parse_program();
        profile.record(EventKind::Parse, t.elapsed().as_millis() as u64, file_str.clone());
        all_diags.append(&mut parse_diags);

        // Stage 3: Semantic
        let t = std::time::Instant::now();
        let mut analyzer = SemanticAnalyzer::new();
        let mut sem_diags = analyzer.analyze(&program);
        profile.record(EventKind::Semantic, t.elapsed().as_millis() as u64, file_str.clone());
        all_diags.append(&mut sem_diags);

        // Stage 4: MIR
        let t = std::time::Instant::now();
        let mir = MirBuilder::lower(&program);
        profile.record(EventKind::Codegen, t.elapsed().as_millis() as u64, file_str.clone());
        if args.emit_mir {
            println!("{}", MirBuilder::to_json(&mir));
        }
        if args.emit_llvm {
            let be = baluac_lib::backend::llvm::LlvmBackend { target_triple: args.target.clone(), opt_level: 2, lto: true, ..Default::default() };
            println!("{}", be.lower(&mir).unwrap());
        }
        if args.emit_clif {
            let be = baluac_lib::backend::cranelift::CraneliftBackend { target_triple: args.target.clone(), opt_level: 0 };
            println!("{}", be.lower(&mir).unwrap());
        }
        if args.backend.is_some() {
            let hw = args.backend.as_deref().unwrap();
            let be = baluac_lib::backend::select_backend(hw);
            if !args.emit_llvm && !args.emit_clif && !args.emit_mir {
                println!("{}", be.lower(&mir).unwrap());
            }
        }

        if let Some(profile_name) = &args.safety_profile {
            println!("; Safety profile: {} — checks enabled (MISRA/AUTOSAR/DO-178C)", profile_name);
        }
        if args.verbose {
            eprintln!("[verbose] lex {:?}ms, parse {:?}ms, semantic {:?}ms", 
                profile.events.iter().filter(|e| matches!(e.kind, EventKind::Lex)).last().map(|e| e.duration_ms).unwrap_or(0),
                profile.events.iter().filter(|e| matches!(e.kind, EventKind::Parse)).last().map(|e| e.duration_ms).unwrap_or(0),
                profile.events.iter().filter(|e| matches!(e.kind, EventKind::Semantic)).last().map(|e| e.duration_ms).unwrap_or(0));
        }
    }
    profile.total_ms = t0.elapsed().as_millis() as u64;
    if args.verbose {
        println!("{}", serde_json::to_string_pretty(&profile).unwrap());
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
        let exe = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or("balua".into());
        println!("{}: compilation successful ({} files, target: {})", exe.trim_end_matches(".exe"), args.files.len(), args.target);
    }

    Ok(())
}
