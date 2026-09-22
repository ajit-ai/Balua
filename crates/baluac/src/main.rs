//! balua — Balua compiler driver (Section 2) — Windows balua.exe
//! Usage: balua hello.bl  |  balua --help  |  balua [--emit-llvm] <file.bl>
//! Legacy alias baluac.exe still works. Package manager is BPM.exe.

use baluac_lib::{diagnostics::*, lexer::Lexer, mir::MirBuilder, parser::Parser, semantic::SemanticAnalyzer, backend::Backend};
use clap::Parser as ClapParser;
use std::path::PathBuf;

#[derive(ClapParser, Debug)]
#[command(name = "balua", version = "v1.0-ga", about = "Balua system programming language compiler — balua.exe (Windows 11). Package manager: BPM.exe. Legacy alias: baluac.exe")]
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

    /// Release mode: Cranelift opt_level 2 (speed). Default is debug (opt 0).
    #[arg(long, default_value_t = false)]
    release: bool,

    /// Emit Cranelift IR (.clif) — lightweight alternative to LLVM
    #[arg(long)]
    emit_clif: bool,

    /// Output artifact path. Without further flags, produces a linked executable.
    #[arg(short = 'o')]
    output: Option<PathBuf>,

    /// Keep the intermediate object file when producing an executable.
    #[arg(long)]
    keep_object: bool,

    /// JSON diagnostics for IDE
    #[arg(long)]
    json_diagnostics: bool,

    /// Safety profile
    #[arg(long)]
    safety_profile: Option<String>,

    /// Optional worst-case stack limit in bytes enforced with --safety-profile
    #[arg(long)]
    safety_stack_limit: Option<usize>,

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
    let mut mir_all: Vec<baluac_lib::mir::MirModule> = Vec::new();
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

        // Stage 3: Semantic (Phase 1 TypeTable, consumed by MIR in Phase 2)
        let t = std::time::Instant::now();
        let mut analyzer = SemanticAnalyzer::new();
        let (mut sem_diags, type_table) = analyzer.analyze(&program);
        let type_entries = type_table.len();
        profile.record(EventKind::Semantic, t.elapsed().as_millis() as u64, file_str.clone());
        all_diags.append(&mut sem_diags);

        // Stage 4: MIR (Phase 2: wired to TypeTable)
        let t = std::time::Instant::now();
        let mir = MirBuilder::lower_with_types(&program, &type_table);
        mir_all.extend(mir.clone());
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

        if args.verbose {
            eprintln!("[verbose] lex {:?}ms, parse {:?}ms, semantic {:?}ms, types {} entries",
                profile.events.iter().filter(|e| matches!(e.kind, EventKind::Lex)).last().map(|e| e.duration_ms).unwrap_or(0),
                profile.events.iter().filter(|e| matches!(e.kind, EventKind::Parse)).last().map(|e| e.duration_ms).unwrap_or(0),
                profile.events.iter().filter(|e| matches!(e.kind, EventKind::Semantic)).last().map(|e| e.duration_ms).unwrap_or(0),
                type_entries);
        }
    }
    // Stage 5 (M2): safety enforcement over all lowered MIR. Violations join
    // diagnostics so they print below and block executable output (fail-closed).
    if args.safety_stack_limit.is_some() && args.safety_profile.is_none() {
        eprintln!("warning: --safety-stack-limit has no effect without --safety-profile");
    }
    if let Some(profile_name) = &args.safety_profile {
        match baluac_lib::safety::parse_profile(profile_name) {
            Some(profile) => {
                let report = baluac_lib::safety::check_modules(&mir_all, &profile, args.safety_stack_limit);
                if args.verbose {
                    eprintln!("{}", report.stack_report);
                }
                if report.errors.is_empty() {
                    eprintln!(
                        "safety profile {}: {} functions checked ({} hardware-targeted skipped), 0 violations",
                        profile_name,
                        report.checked_functions,
                        report.skipped_hw.len()
                    );
                }
                all_diags.extend(report.errors);
            }
            None => all_diags.push(baluac_lib::safety::unknown_profile_error(profile_name)),
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
    } else if let Some(out) = &args.output {
        // Phase 2: produce a linked executable via the Cranelift backend.
        // Debug default opt 0; --release selects opt 2 (speed).
        let opt: u8 = if args.release { 2 } else { 0 };
        let summary = baluac_lib::backend::object_emit::build_executable(
            &mir_all,
            out,
            opt,
            args.keep_object,
        )?;
        let _ = summary;
        println!("balua: produced executable {}", out.display());
    } else if !args.emit_llvm && !args.emit_mir {
        let exe = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or("balua".into());
        println!("{}: compilation successful ({} files, target: {})", exe.trim_end_matches(".exe"), args.files.len(), args.target);
    }

    Ok(())
}
