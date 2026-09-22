//! LLVM IR backend — CPU targets (x86_64, aarch64, riscv64, arm-none-eabi)
//! Two modes: string-builder IR (default, no LLVM linked) and real codegen
//! via llvm-sys 191 (LLVM 19.1.x) behind `--features llvm`.
//!
//! Real mode lowers MIR with an alloca model (every virtual register gets an
//! entry-block `alloca`; defs store, uses load), which is correct for any CFG
//! without PHI construction. Integer domain only (`i32`, mirroring the
//! Cranelift backend): comparisons widen via `zext`, floats/strings lower to
//! zero (documented limitation, same as Cranelift today). `opt_level` maps to
//! the LLVM codegen level; there are no MIR-level optimization passes yet.
//! `lto`/`pgo`/`bolt` fields are accepted but not implemented (recorded for
//! post-E1 work).
//!
//! Requires LLVM 19 on PATH or `LLVM_SYS_191_PREFIX` at build time.

use super::Backend;
use crate::mir::{MirModule, Instruction, Terminator};

#[derive(Default)]
pub struct LlvmBackend {
    pub target_triple: String,
    pub opt_level: u8, // 0-3
    pub lto: bool,
    pub pgo: bool,
    pub bolt: bool,
}

impl Backend for LlvmBackend {
    fn name(&self) -> &'static str { "llvm" }

    fn lower(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        #[cfg(feature = "llvm")]
        return self.lower_llvm_sys(modules);
        #[cfg(not(feature = "llvm"))]
        return self.lower_string(modules);
    }
}

impl LlvmBackend {
    #[cfg(feature = "llvm")]
    fn lower_llvm_sys(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        // Real llvm-sys 191 codegen (LLVM 19.1.x): build an in-memory module
        // from MIR and print it. Requires LLVM 19 on PATH or
        // LLVM_SYS_191_PREFIX at build time.
        let (ctx, module) = sys::build_module(modules)?;
        let text = sys::print_module(module);
        unsafe {
            llvm_sys::core::LLVMDisposeModule(module);
            llvm_sys::core::LLVMContextDispose(ctx);
        }
        Ok(text)
    }

    #[cfg(not(feature = "llvm"))]
    fn lower_string(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        let triple = if self.target_triple.is_empty() { "x86_64-pc-windows-msvc" } else { &self.target_triple };
        let mut out = String::new();
        out.push_str(&format!("; Balua LLVM backend — target: {} (opt={}, lto={}, pgo={}, bolt={})\n", triple, self.opt_level, self.lto, self.pgo, self.bolt));
        out.push_str("; Generated from Balua MIR — SSA, hw-annotated BBs, typed\n");
        out.push_str(&format!("target triple = \"{}\"\n", triple));
        out.push_str(match triple {
            t if t.contains("aarch64") => "target datalayout = \"e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128\"\n",
            t if t.contains("riscv64") => "target datalayout = \"e-m:e-p:64:64-i64:64-i128:128-n32:64-S128\"\n",
            _ => "target datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n",
        });
        if self.lto { out.push_str("; LTO: thin (4GB host) — via -Clto=thin / -Ccodegen-units=16\n"); }
        if self.pgo { out.push_str("; PGO: instrumentation enabled — balua-prof will merge profdata\n"); }
        if self.bolt { out.push_str("; BOLT: post-link enabled — requires llvm-bolt\n"); }
        out.push_str("; llvm-sys not linked (feature llvm off) — string-builder IR for 4GB host, enable via --features llvm\n");
        for m in modules {
            out.push_str(&format!("\n; Module: {}\n", m.name));
            for f in &m.functions {
                if let Some(hw) = &f.hardware {
                    out.push_str(&format!("; hw: {:?}\n", hw));
                }
                let ret_ty = "i32";
                out.push_str(&format!("define {} @{}() {{\n", ret_ty, f.name));
                out.push_str("entry:\n");
                for bb in &f.basic_blocks {
                    if bb.id != 0 {
                        out.push_str(&format!("bb{}: ; hw={:?}\n", bb.id, bb.hardware));
                    }
                    for inst in &bb.instructions {
                        out.push_str(&format!("  ; {}\n", llvm_inst(inst)));
                    }
                    match &bb.terminator {
                        Terminator::Return(Some(v)) => out.push_str(&format!("  ret i32 {}\n", v)),
                        Terminator::Return(None) => out.push_str("  ret i32 0\n"),
                        Terminator::Jump(t) => out.push_str(&format!("  br label %bb{}\n", t)),
                        Terminator::Branch { cond, then_bb, else_bb } => out.push_str(&format!("  br i1 {}, label %bb{}, label %bb{}\n", cond, then_bb, else_bb)),
                        Terminator::Unreachable => out.push_str("  unreachable\n"),
                    }
                }
                out.push_str("}\n");
            }
        }
        out.push_str("\n; Windows 11 x64 SEH — balua.exe uses pc-windows-msvc triple\n");
        Ok(out)
    }
}

/// Parse an integer constant from a MIR `const.N` op (`None` for
/// `constf.`/`consts.` and anything else). Shared by the real backend;
/// unit-tested without LLVM linked.
pub(crate) fn mir_const_int(op: &str) -> Option<i64> {
    op.strip_prefix("const.").and_then(|n| n.parse::<i64>().ok())
}

/// Compile MIR to a native object file via real LLVM (feature `llvm`).
/// Falls back to a clear error when built without the feature.
#[cfg(feature = "llvm")]
pub fn compile_llvm_to_object(
    modules: &[MirModule],
    out_obj: &std::path::Path,
    target_triple: &str,
    opt_level: u8,
) -> anyhow::Result<String> {
    sys::emit_object(modules, out_obj, target_triple, opt_level)
}

/// Full pipeline: MIR -> LLVM object -> linked executable (feature `llvm`).
#[cfg(feature = "llvm")]
pub fn build_executable_llvm(
    modules: &[MirModule],
    out_exe: &std::path::Path,
    target_triple: &str,
    opt_level: u8,
    keep_object: bool,
) -> anyhow::Result<std::path::PathBuf> {
    let mut obj = out_exe.as_os_str().to_owned();
    obj.push(".o");
    let obj = std::path::PathBuf::from(obj);
    compile_llvm_to_object(modules, &obj, target_triple, opt_level)?;
    let exe = super::object_emit::ensure_exe_extension(out_exe);
    super::object_emit::link_executable(&obj, &exe)?;
    if !keep_object {
        let _ = std::fs::remove_file(&obj);
    }
    Ok(exe)
}

/// Fallback when built without `--features llvm`: fail closed with guidance
/// instead of silently producing a Cranelift binary.
#[cfg(not(feature = "llvm"))]
pub fn build_executable_llvm(
    _modules: &[MirModule],
    _out_exe: &std::path::Path,
    _target_triple: &str,
    _opt_level: u8,
    _keep_object: bool,
) -> anyhow::Result<std::path::PathBuf> {
    Err(anyhow::anyhow!(
        "--cpu-backend llvm requires building with --features llvm (LLVM 19.1.x via LLVM_SYS_191_PREFIX or llvm-config); use --cpu-backend cranelift"
    ))
}

/// Real LLVM lowering (feature `llvm`, llvm-sys 191). Alloca model: every
/// virtual register gets an entry-block `alloca`; definitions store, uses
/// load. Uses only long-stable C API functions.
#[cfg(feature = "llvm")]
mod sys {
    use super::mir_const_int;
    use crate::mir::{Instruction, MirFunction, MirModule, Terminator};
    use llvm_sys::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
    use llvm_sys::core::*;
    use llvm_sys::prelude::*;
    use llvm_sys::target::*;
    use llvm_sys::target_machine::*;
    use std::collections::HashMap;
    use std::ffi::{CStr, CString};

    fn c(s: &str) -> CString {
        CString::new(s).unwrap_or_default()
    }

    fn cstr(ptr: *mut std::os::raw::c_char) -> String {
        if ptr.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    }

    struct FnCx {
        blocks: HashMap<usize, LLVMBasicBlockRef>,
        slots: HashMap<String, LLVMValueRef>,
        i32t: LLVMTypeRef,
    }

    /// Build an in-memory LLVM module. Returns owned (context, module).
    pub(super) fn build_module(modules: &[MirModule]) -> anyhow::Result<(LLVMContextRef, LLVMModuleRef)> {
        unsafe {
            let ctx = LLVMContextCreate();
            let i32t = LLVMInt32TypeInContext(ctx);
            let module = LLVMModuleCreateWithNameInContext(c("balua").as_ptr(), ctx);
            // Pass 1: declare every function so calls resolve.
            let mut decls: HashMap<String, (LLVMValueRef, LLVMTypeRef)> = HashMap::new();
            for m in modules {
                for f in &m.functions {
                    if f.basic_blocks.is_empty() {
                        continue;
                    }
                    let mut params = vec![i32t; f.params.len()];
                    let fty = LLVMFunctionType(i32t, params.as_mut_ptr(), params.len() as u32, 0);
                    let func = LLVMAddFunction(module, c(&f.name).as_ptr(), fty);
                    decls.insert(f.name.clone(), (func, fty));
                }
            }
            // Pass 2: define bodies.
            for m in modules {
                for f in &m.functions {
                    if f.basic_blocks.is_empty() {
                        continue;
                    }
                    let (func, _) = decls.get(&f.name).copied().ok_or_else(|| anyhow::anyhow!("missing decl for {}", f.name))?;
                    emit_body(ctx, i32t, f, func, &decls)?;
                }
            }
            let mut verify_msg: *mut std::os::raw::c_char = std::ptr::null_mut();
            if LLVMVerifyModule(
                module,
                LLVMVerifierFailureAction::LLVMReturnStatusAction,
                &mut verify_msg,
            ) != 0
            {
                let msg = cstr(verify_msg);
                LLVMDisposeMessage(verify_msg);
                LLVMDisposeModule(module);
                LLVMContextDispose(ctx);
                anyhow::bail!("LLVM module verification failed: {}", msg);
            }
            Ok((ctx, module))
        }
    }

    fn slot<'a>(slots: &'a mut HashMap<String, LLVMValueRef>, builder: LLVMBuilderRef, i32t: LLVMTypeRef, reg: &str) -> LLVMValueRef {
        if let Some(&v) = slots.get(reg) {
            return v;
        }
        unsafe {
            let a = LLVMBuildAlloca(builder, i32t, c(reg).as_ptr());
            slots.insert(reg.to_string(), a);
            a
        }
    }

    fn load(builder: LLVMBuilderRef, slots: &mut HashMap<String, LLVMValueRef>, i32t: LLVMTypeRef, reg: &str) -> LLVMValueRef {
        unsafe {
            let ptr = slot(slots, builder, i32t, reg);
            LLVMBuildLoad2(builder, i32t, ptr, c("").as_ptr())
        }
    }

    fn store(builder: LLVMBuilderRef, slots: &mut HashMap<String, LLVMValueRef>, i32t: LLVMTypeRef, dest: &str, val: LLVMValueRef) {
        unsafe {
            let ptr = slot(slots, builder, i32t, dest);
            LLVMBuildStore(builder, val, ptr);
        }
    }

    fn iconst(builder: LLVMBuilderRef, i32t: LLVMTypeRef, v: i64) -> LLVMValueRef {
        unsafe { LLVMConstInt(i32t, v as u64, 1) }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_body(
        ctx: LLVMContextRef,
        i32t: LLVMTypeRef,
        f: &MirFunction,
        func: LLVMValueRef,
        decls: &HashMap<String, (LLVMValueRef, LLVMTypeRef)>,
    ) -> anyhow::Result<()> {
        unsafe {
            let builder = LLVMCreateBuilderInContext(ctx);
            let mut cx = FnCx { blocks: HashMap::new(), slots: HashMap::new(), i32t };
            for bb in &f.basic_blocks {
                let name = if bb.id == 0 { "entry".to_string() } else { format!("bb{}", bb.id) };
                cx.blocks.insert(bb.id, LLVMAppendBasicBlockInContext(ctx, func, c(&name).as_ptr()));
            }
            let entry = *cx.blocks.get(&0).ok_or_else(|| anyhow::anyhow!("no entry block in {}", f.name))?;
            // Pre-create one alloca per dest register plus params, all in entry.
            LLVMPositionBuilderAtEnd(builder, entry);
            for (i, _) in f.params.iter().enumerate() {
                let alloca = LLVMBuildAlloca(builder, i32t, c(&format!("p{}", i)).as_ptr());
                let incoming = LLVMGetParam(func, i as u32);
                LLVMBuildStore(builder, incoming, alloca);
                cx.slots.insert(format!("p{}", i), alloca);
            }
            let mut dests: Vec<String> = Vec::new();
            for bb in &f.basic_blocks {
                for inst in &bb.instructions {
                    match inst {
                        Instruction::Alloca { dest, .. }
                        | Instruction::Load { dest, .. }
                        | Instruction::BinOp { dest, .. }
                        | Instruction::MoveToDevice { dest, .. }
                        | Instruction::Phi { dest, .. } => dests.push(dest.clone()),
                        Instruction::Call { dest, .. } => {
                            if let Some(d) = dest {
                                dests.push(d.clone());
                            }
                        }
                        Instruction::HwIntrinsic { dest, .. } => {
                            if let Some(d) = dest {
                                dests.push(d.clone());
                            }
                        }
                        Instruction::Store { .. } => {}
                    }
                }
            }
            dests.sort();
            dests.dedup();
            for d in &dests {
                if !cx.slots.contains_key(d) {
                    let alloca = LLVMBuildAlloca(builder, i32t, c(d).as_ptr());
                    cx.slots.insert(d.clone(), alloca);
                }
            }
            // Emit.
            for bb in &f.basic_blocks {
                let block = *cx.blocks.get(&bb.id).ok_or_else(|| anyhow::anyhow!("missing block {}", bb.id))?;
                LLVMPositionBuilderAtEnd(builder, block);
                for inst in &bb.instructions {
                    emit_inst(builder, &mut cx, inst, decls)?;
                }
                // Copy the block map: terminator emission borrows it while
                // `cx` is borrowed mutably for slot access.
                let blocks = cx.blocks.clone();
                let term = bb.terminator.clone();
                emit_term(builder, &mut cx, &term, &blocks)?;
            }
            LLVMDisposeBuilder(builder);
            Ok(())
        }
    }

    fn emit_inst(
        builder: LLVMBuilderRef,
        cx: &mut FnCx,
        inst: &Instruction,
        decls: &HashMap<String, (LLVMValueRef, LLVMTypeRef)>,
    ) -> anyhow::Result<()> {
        unsafe {
            match inst {
                Instruction::Alloca { .. } => {}
                Instruction::Load { dest, src } => {
                    let v = load(builder, &mut cx.slots, cx.i32t, src);
                    store(builder, &mut cx.slots, cx.i32t, dest, v);
                }
                Instruction::Store { val, ptr } => {
                    let v = load(builder, &mut cx.slots, cx.i32t, val);
                    store(builder, &mut cx.slots, cx.i32t, ptr, v);
                }
                Instruction::BinOp { dest, op, lhs, rhs } => {
                    emit_binop(builder, cx, dest, op, lhs, rhs)?;
                }
                Instruction::Call { dest, callee, args } => {
                    let name = callee.trim_start_matches(['@', '%']);
                    let (callee_fn, fty) = decls.get(name).copied().ok_or_else(|| anyhow::anyhow!("call to undeclared function '{}'", name))?;
                    let mut arg_vals: Vec<LLVMValueRef> =
                        args.iter().map(|a| load(builder, &mut cx.slots, cx.i32t, a)).collect();
                    let r = LLVMBuildCall2(builder, fty, callee_fn, arg_vals.as_mut_ptr(), arg_vals.len() as u32, c("").as_ptr());
                    if let Some(d) = dest {
                        store(builder, &mut cx.slots, cx.i32t, d, r);
                    }
                }
                Instruction::HwIntrinsic { dest, .. } | Instruction::MoveToDevice { dest, .. } => {
                    // No CPU lowering: zero-fill like the Cranelift backend.
                    if let Some(d) = dest {
                        let z = iconst(builder, cx.i32t, 0);
                        store(builder, &mut cx.slots, cx.i32t, d, z);
                    }
                }
                Instruction::Phi { dest, .. } => {
                    // Unused by MIR lowering; keep the slot defined.
                    let z = iconst(builder, cx.i32t, 0);
                    store(builder, &mut cx.slots, cx.i32t, dest, z);
                }
            }
            Ok(())
        }
    }

    fn emit_binop(
        builder: LLVMBuilderRef,
        cx: &mut FnCx,
        dest: &str,
        op: &str,
        lhs: &str,
        rhs: &str,
    ) -> anyhow::Result<()> {
        unsafe {
            if let Some(n) = mir_const_int(op) {
                let v = iconst(builder, cx.i32t, n);
                store(builder, &mut cx.slots, cx.i32t, dest, v);
                return Ok(());
            }
            if op == "id" {
                let v = load(builder, &mut cx.slots, cx.i32t, lhs);
                store(builder, &mut cx.slots, cx.i32t, dest, v);
                return Ok(());
            }
            // Non-constant strings/floats have no integer lowering; zero-fill.
            if op.starts_with("const") {
                let z = iconst(builder, cx.i32t, 0);
                store(builder, &mut cx.slots, cx.i32t, dest, z);
                return Ok(());
            }
            let a = load(builder, &mut cx.slots, cx.i32t, lhs);
            let b = load(builder, &mut cx.slots, cx.i32t, rhs);
            let v = match op {
                "+" => LLVMBuildAdd(builder, a, b, c("").as_ptr()),
                "-" => LLVMBuildSub(builder, a, b, c("").as_ptr()),
                "*" => LLVMBuildMul(builder, a, b, c("").as_ptr()),
                "/" => LLVMBuildSDiv(builder, a, b, c("").as_ptr()),
                "%" => LLVMBuildSRem(builder, a, b, c("").as_ptr()),
                "&" => LLVMBuildAnd(builder, a, b, c("").as_ptr()),
                "|" => LLVMBuildOr(builder, a, b, c("").as_ptr()),
                "^" => LLVMBuildXor(builder, a, b, c("").as_ptr()),
                "<<" => LLVMBuildShl(builder, a, b, c("").as_ptr()),
                ">>" => LLVMBuildAShr(builder, a, b, c("").as_ptr()),
                "==" => bool_to_i32(builder, cx.i32t, LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntEQ, a, b, c("").as_ptr())),
                "!=" => bool_to_i32(builder, cx.i32t, LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntNE, a, b, c("").as_ptr())),
                "<" => bool_to_i32(builder, cx.i32t, LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSLT, a, b, c("").as_ptr())),
                ">" => bool_to_i32(builder, cx.i32t, LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSGT, a, b, c("").as_ptr())),
                "<=" => bool_to_i32(builder, cx.i32t, LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSLE, a, b, c("").as_ptr())),
                ">=" => bool_to_i32(builder, cx.i32t, LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSGE, a, b, c("").as_ptr())),
                "&&" => {
                    let za = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntNE, a, iconst(builder, cx.i32t, 0), c("").as_ptr());
                    let zb = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntNE, b, iconst(builder, cx.i32t, 0), c("").as_ptr());
                    bool_to_i32(builder, cx.i32t, LLVMBuildAnd(builder, za, zb, c("").as_ptr()))
                }
                "||" => {
                    let za = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntNE, a, iconst(builder, cx.i32t, 0), c("").as_ptr());
                    let zb = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntNE, b, iconst(builder, cx.i32t, 0), c("").as_ptr());
                    bool_to_i32(builder, cx.i32t, LLVMBuildOr(builder, za, zb, c("").as_ptr()))
                }
                // Unknown operator: pass lhs through (same quirk as Cranelift).
                _ => a,
            };
            store(builder, &mut cx.slots, cx.i32t, dest, v);
            Ok(())
        }
    }

    fn bool_to_i32(builder: LLVMBuilderRef, i32t: LLVMTypeRef, b1: LLVMValueRef) -> LLVMValueRef {
        unsafe { LLVMBuildZExt(builder, b1, i32t, c("").as_ptr()) }
    }

    fn emit_term(
        builder: LLVMBuilderRef,
        cx: &mut FnCx,
        term: &Terminator,
        blocks: &HashMap<usize, LLVMBasicBlockRef>,
    ) -> anyhow::Result<()> {
        unsafe {
            match term {
                Terminator::Return(Some(reg)) => {
                    let v = load(builder, &mut cx.slots, cx.i32t, reg);
                    LLVMBuildRet(builder, v);
                }
                Terminator::Return(None) => {
                    LLVMBuildRet(builder, iconst(builder, cx.i32t, 0));
                }
                Terminator::Jump(t) => {
                    let b = *blocks.get(t).ok_or_else(|| anyhow::anyhow!("jump to missing block {}", t))?;
                    LLVMBuildBr(builder, b);
                }
                Terminator::Branch { cond, then_bb, else_bb } => {
                    let c = load(builder, &mut cx.slots, cx.i32t, cond);
                    let nz = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntNE, c, iconst(builder, cx.i32t, 0), c("").as_ptr());
                    let t = *blocks.get(then_bb).ok_or_else(|| anyhow::anyhow!("branch then missing {}", then_bb))?;
                    let e = *blocks.get(else_bb).ok_or_else(|| anyhow::anyhow!("branch else missing {}", else_bb))?;
                    LLVMBuildCondBr(builder, nz, t, e);
                }
                Terminator::Unreachable => {
                    LLVMBuildUnreachable(builder);
                }
            }
            Ok(())
        }
    }

    /// Print the built module as LLVM IR text.
    pub(super) fn print_module(module: LLVMModuleRef) -> String {
        unsafe {
            let ptr = LLVMPrintModuleToString(module);
            let text = cstr(ptr);
            LLVMDisposeMessage(ptr);
            text
        }
    }

    /// Build the module and emit a native object file with the host-target
    /// machine (or the requested triple). `opt_level`: 0 = none, 1 = less,
    /// 2 = default, 3+ = aggressive.
    pub(super) fn emit_object(
        modules: &[MirModule],
        out_obj: &std::path::Path,
        target_triple: &str,
        opt_level: u8,
    ) -> anyhow::Result<String> {
        unsafe {
            LLVM_InitializeAllTargetInfos();
            LLVM_InitializeAllTargets();
            LLVM_InitializeAllTargetMCs();
            LLVM_InitializeAllAsmPrinters();
            LLVM_InitializeAllAsmParsers();
            let (ctx, module) = build_module(modules)?;
            let default_triple = LLVMGetDefaultTargetTriple();
            let triple = if target_triple.is_empty() { cstr(default_triple) } else { target_triple.to_string() };
            LLVMDisposeMessage(default_triple);
            LLVMSetTarget(module, c(&triple).as_ptr());
            let mut target: LLVMTargetRef = std::ptr::null_mut();
            let mut err: *mut std::os::raw::c_char = std::ptr::null_mut();
            if LLVMGetTargetFromTriple(c(&triple).as_ptr(), &mut target, &mut err) != 0 {
                let msg = cstr(err);
                LLVMDisposeMessage(err);
                LLVMDisposeModule(module);
                LLVMContextDispose(ctx);
                anyhow::bail!("no LLVM target for '{}': {}", triple, msg);
            }
            let level = match opt_level {
                0 => LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
                1 => LLVMCodeGenOptLevel::LLVMCodeGenLevelLess,
                2 => LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
                _ => LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
            };
            let machine = LLVMCreateTargetMachine(
                target,
                c(&triple).as_ptr(),
                c("generic").as_ptr(),
                c("").as_ptr(),
                level,
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );
            if machine.is_null() {
                LLVMDisposeModule(module);
                LLVMContextDispose(ctx);
                anyhow::bail!("could not create LLVM target machine for '{}'", triple);
            }
            let out = c(&out_obj.display().to_string());
            let mut emit_err: *mut std::os::raw::c_char = std::ptr::null_mut();
            let failed = LLVMTargetMachineEmitToFile(
                machine,
                module,
                out.as_ptr(),
                LLVMCodeGenFileType::LLVMObjectFile,
                &mut emit_err,
            );
            if failed != 0 {
                let msg = cstr(emit_err);
                LLVMDisposeMessage(emit_err);
                LLVMDisposeTargetMachine(machine);
                LLVMDisposeModule(module);
                LLVMContextDispose(ctx);
                anyhow::bail!("LLVM object emission failed: {}", msg);
            }
            LLVMDisposeTargetMachine(machine);
            LLVMDisposeModule(module);
            LLVMContextDispose(ctx);
            let n: usize = modules.iter().map(|m| m.functions.len()).sum();
            Ok(format!("emitted {} functions to {}", n, out_obj.display()))
        }
    }
}

fn llvm_inst(inst: &Instruction) -> String {
    match inst {
        Instruction::Alloca { dest, ty } => format!("{} = alloca {}, align 8", dest, ty),
        Instruction::Load { dest, src } => format!("{} = load i32, ptr {}, align 4", dest, src),
        Instruction::Store { val, ptr } => format!("store i32 {}, ptr {}, align 4", val, ptr),
        Instruction::BinOp { dest, op, lhs, rhs } => {
            let llvm_op = match op.as_str() { "+" => "add", "-" => "sub", "*" => "mul", "/" => "sdiv", _ => "add" };
            format!("{} = {} i32 {}, {}", dest, llvm_op, lhs, rhs)
        }
        Instruction::Call { dest, callee, args } => {
            let d = dest.as_ref().map(|d| format!("{} = ", d)).unwrap_or_default();
            format!("{}call i32 @{}({})", d, callee, args.join(", "))
        }
        Instruction::HwIntrinsic { dest, name, args } => format!("; hw intrinsic {}({}) -> {:?}", name, args.join(", "), dest),
        Instruction::MoveToDevice { dest, src } => format!("; move_to_device {} = {}", dest, src),
        Instruction::Phi { dest, incoming } => format!("{} = phi i32 [ {} ]", dest, incoming.iter().map(|(v,bb)| format!("{}, %bb{}", v, bb)).collect::<Vec<_>>().join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{BasicBlock, MirFunction, Terminator};
    use crate::diagnostics::Span;
    #[test]
    fn emits_typed_ir() {
        let m = MirModule { name: "test".into(), functions: vec![MirFunction {
            name: "add".into(), hardware: None, params: vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
            basic_blocks: vec![BasicBlock { id: 0, hardware: None, instructions: vec![Instruction::BinOp { dest: "%0".into(), op: "+".into(), lhs: "%a".into(), rhs: "%b".into() }], terminator: Terminator::Return(Some("%0".into())) }],
            attrs: vec![],
            span: Span { file: "t".into(), line:1, col:1, end_line:1, end_col:1 }
        }] };
        let ir = LlvmBackend { target_triple: "x86_64-pc-windows-msvc".into(), opt_level: 2, lto: true, ..Default::default() }.lower(&[m]).unwrap();
        assert!(ir.contains("target triple"));
        assert!(ir.contains("define i32 @add"));
        assert!(ir.contains("add i32"));
    }
    #[test]
    fn mir_const_int_parses() {
        assert_eq!(super::mir_const_int("const.42"), Some(42));
        assert_eq!(super::mir_const_int("const.-7"), Some(-7));
        assert_eq!(super::mir_const_int("const.0"), Some(0));
        assert_eq!(super::mir_const_int("constf.1.5"), None);
        assert_eq!(super::mir_const_int("id"), None);
        assert_eq!(super::mir_const_int("+"), None);
    }
}
