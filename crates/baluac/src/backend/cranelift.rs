//! Cranelift backend — real native code generation (Phase 2).
//!
//! Lowers MIR to Cranelift IR (CLIF) and emits a native relocatable object file
//! via cranelift-object, which the driver then links with the host C toolchain.
//!
//! Design: each MIR virtual register maps to a Cranelift SSA `Variable` (the
//! frontend manages phis at block joins). Arithmetic uses i32; comparisons
//! produce b1 (and are widened to i32 via uextend when used as values, or used
//! directly as `brif` conditions).

use super::Backend;
use crate::mir::{Instruction, MirModule, Terminator};
use anyhow::{anyhow, Result};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    types, AbiParam, Block, Function, InstBuilder, Signature, UserFuncName,
};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{isa, Context};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
pub struct CraneliftBackend {
    pub target_triple: String,
    pub opt_level: u8,
}

impl Backend for CraneliftBackend {
    fn name(&self) -> &'static str {
        "cranelift"
    }

    fn lower(&self, modules: &[MirModule]) -> Result<String> {
        // The choke point used by the generic Backend trait: emit to a temp
        // object and report (no linking here — object_emit handles the full
        // build). This keeps --emit-clif-style text output usable.
        let tmp = std::env::temp_dir().join(format!("balua_{}.o", std::process::id()));
        let summary = compile_modules_to_object(modules, &tmp, self.opt_level)?;
        let _ = std::fs::remove_file(&tmp);
        Ok(summary)
    }
}

/// Compile all MIR modules into a native object file at `out_path`.
/// Returns a short human-readable summary.
pub fn compile_modules_to_object(
    modules: &[MirModule],
    out_path: &Path,
    opt_level: u8,
) -> Result<String> {
    let mut sb = settings::builder();
    sb.set(
        "opt_level",
        if opt_level >= 2 { "speed" } else { "none" },
    )
    .map_err(|e| anyhow!("failed to set opt_level: {}", e))?;
    let flags = settings::Flags::new(sb);
    let triple = self_target_triple();
    let isa_builder = isa::lookup_by_name(&triple)
        .map_err(|e| anyhow!("failed to lookup target ISA '{}': {}", triple, e))?;
    let isa = isa_builder
        .finish(flags.clone())
        .map_err(|e| anyhow!("failed to finish target ISA: {}", e))?;

    let mut obj = ObjectModule::new(
        ObjectBuilder::new(isa, "balua", default_libcall_names())
            .map_err(|e| anyhow!("failed to init object builder: {}", e))?,
    );

    // Pass 1: declare every function so cross-module calls resolve.
    let mut ids: HashMap<String, cranelift_module::FuncId> = HashMap::new();
    let mut sigs: HashMap<String, Signature> = HashMap::new();
    for m in modules {
        for f in &m.functions {
            if f.basic_blocks.is_empty() {
                continue;
            }
            let sig = function_signature(f);
            let id = obj
                .declare_function(&f.name, Linkage::Export, &sig)
                .map_err(|e| anyhow!("declare {}: {}", f.name, e))?;
            ids.insert(f.name.clone(), id);
            sigs.insert(f.name.clone(), sig);
        }
    }

    // Pass 2: define each function body.
    for m in modules {
        for f in &m.functions {
            if f.basic_blocks.is_empty() {
                continue;
            }
            let id = *ids.get(&f.name).ok_or_else(|| anyhow!("missing id for {}", f.name))?;
            let sig = sigs.get(&f.name).unwrap().clone();
            let mut ctx = Context::new();
            ctx.func = Function::with_name_signature(UserFuncName::user(0, id.as_u32()), sig);
            emit_body(f, &mut ctx, &mut obj, &ids)?;
            obj.define_function(id, &mut ctx)
                .map_err(|e| anyhow!("define {}: {}", f.name, e))?;
        }
    }

    let product = obj.finish();
    let bytes = product.emit().map_err(|e| anyhow!("emit object: {}", e))?;
    std::fs::write(out_path, &bytes).map_err(|e| anyhow!("write {}: {}", out_path.display(), e))?;

    let n: usize = modules.iter().map(|m| m.functions.len()).sum();
    Ok(format!("emitted {} functions to {}", n, out_path.display()))
}

fn self_target_triple() -> String {
    if let Ok(t) = std::env::var("BALUA_TARGET_TRIPLE") {
        if !t.is_empty() {
            return t;
        }
    }
    target_lexicon::Triple::host().to_string()
}

/// Balua functions are `fn(a: i32...) -> i32` (the unit/void return uses 0).
fn function_signature(f: &crate::mir::MirFunction) -> Signature {
    let mut sig = Signature::new(CallConv::SystemV);
    for _ in &f.params {
        sig.params.push(AbiParam::new(types::I32));
    }
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

fn emit_body(
    f: &crate::mir::MirFunction,
    ctx: &mut Context,
    obj: &mut ObjectModule,
    ids: &HashMap<String, cranelift_module::FuncId>,
) -> Result<()> {
    let mut func_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

    // Pre-create all MIR blocks as CLIF blocks.
    let mut clif_block: HashMap<usize, Block> = HashMap::new();
    for bb in &f.basic_blocks {
        clif_block.insert(bb.id, builder.create_block());
    }
    let entry = clif_block
        .get(&0)
        .copied()
        .ok_or_else(|| anyhow!("no entry block in function {}", f.name))?;
    builder.switch_to_block(entry);

    // Declare a fixed pool of I32 variables so any MIR register can be used as
    // an operand without a separate declare_var at its first (possibly use-only)
    // appearance. This avoids "variable is used but its type has not been
    // declared" panics across phi/join structure.
    const VAR_POOL: u32 = 4096;
    for i in 1..=VAR_POOL {
        builder.declare_var(Variable::from_u32(i), types::I32);
    }

    // Bind parameters to variables, keyed by their MIR register name "p<i>"
    // so the frontend reads of Ident(param) (which lower to %p<i>) resolve to
    // the incoming block-param value rather than an uninitialized variable.
    let mut var_of: HashMap<String, Variable> = HashMap::new();
    for (i, (_pname, _pty)) in f.params.iter().enumerate() {
        let v = Variable::from_u32(i as u32 + 1);
        let val = builder.append_block_param(entry, types::I32);
        builder.def_var(v, val);
        var_of.insert(format!("p{}", i), v);
    }
    let mut next_var = f.params.len() as u32 + 1;

    // Emit each block.
    for bb in &f.basic_blocks {
        let block = *clif_block
            .get(&bb.id)
            .ok_or_else(|| anyhow!("missing block {}", bb.id))?;
        builder.switch_to_block(block);
        for inst in &bb.instructions {
            emit_inst(inst, &mut builder, &mut var_of, &mut next_var, obj, ids, &f.params)?;
        }
        emit_terminator(&bb.terminator, &mut builder, &mut var_of, &clif_block, &mut next_var)?;
    }
    // Seal all blocks only after the whole function is built so that loop
    // headers can receive their back-edge predecessors before being sealed.
    builder.seal_all_blocks();
    builder.finalize();
    Ok(())
}

type ClifBuilder<'a> = FunctionBuilder<'a>;

fn var_name<'a>(reg: &str) -> String {
    reg.trim_start_matches('%').to_string()
}

fn get_var(var_of: &mut HashMap<String, Variable>, name: &str, next: &mut u32) -> Variable {
    if let Some(v) = var_of.get(name) {
        *v
    } else {
        *next += 1;
        let v = Variable::from_u32(*next);
        var_of.insert(name.to_string(), v);
        v
    }
}

fn emit_inst(
    inst: &Instruction,
    builder: &mut ClifBuilder,
    var_of: &mut HashMap<String, Variable>,
    next_var: &mut u32,
    obj: &mut ObjectModule,
    ids: &HashMap<String, cranelift_module::FuncId>,
    _params: &[(String, String)],
) -> Result<()> {
    match inst {
        Instruction::BinOp { dest, op, lhs, rhs } => {
            emit_binop(dest, op, lhs, rhs, builder, var_of, next_var)?;
        }
        Instruction::Call { dest, callee, args } => {
            emit_call(dest, callee, args, builder, var_of, next_var, obj, ids)?;
        }
        Instruction::Load { dest, src } => {
            let v = use_var(builder, get_var(var_of, &var_name(src), next_var));
            let d = get_var(var_of, &var_name(dest), next_var);
            builder.def_var(d, v);
        }
        Instruction::Store { val, ptr } => {
            let v = use_var(builder, get_var(var_of, &var_name(val), next_var));
            let d = get_var(var_of, &var_name(ptr), next_var);
            builder.def_var(d, v);
        }
        Instruction::Alloca { dest, .. } => {
            let d = get_var(var_of, &var_name(dest), next_var);
            let zero = builder.ins().iconst(types::I32, 0);
            builder.def_var(d, zero);
        }
        Instruction::Phi { dest, .. } => {
            // Variables already model phis; nothing extra needed.
            let _ = get_var(var_of, &var_name(dest), next_var);
        }
        Instruction::HwIntrinsic { dest, .. } => {
            // Hardware operations are unsupported on the CPU target.
            unsafe_slot(dest.as_deref(), builder, var_of, next_var);
        }
        Instruction::MoveToDevice { dest, .. } => {
            // Hardware operations are unsupported on the CPU target.
            unsafe_slot(Some(dest), builder, var_of, next_var);
        }
    }
    Ok(())
}

fn unsafe_slot(
    dest: Option<&str>,
    builder: &mut ClifBuilder,
    var_of: &mut HashMap<String, Variable>,
    next_var: &mut u32,
) {
    let d = dest.map(var_name).unwrap_or_else(|| format!("__unk{}", *next_var));
    let v = get_var(var_of, &d, next_var);
    let zero = builder.ins().iconst(types::I32, 0);
    builder.def_var(v, zero);
}

fn use_var(
    builder: &mut FunctionBuilder,
    v: Variable,
) -> cranelift_codegen::ir::Value {
    builder.use_var(v)
}

fn emit_binop(
    dest: &str,
    op: &str,
    lhs: &str,
    rhs: &str,
    builder: &mut ClifBuilder,
    var_of: &mut HashMap<String, Variable>,
    next_var: &mut u32,
) -> Result<()> {
    let d = get_var(var_of, &var_name(dest), next_var);

    if let Some(n) = op.strip_prefix("const.") {
        let i: i32 = n.parse().unwrap_or(0);
        let v = builder.ins().iconst(types::I32, i as i64);
        builder.def_var(d, v);
        return Ok(());
    }
    if let Some(n) = op.strip_prefix("constf.") {
        let f: f32 = n.parse().unwrap_or(0.0);
        let v = builder.ins().f32const(f);
        builder.def_var(d, v);
        return Ok(());
    }
    if let Some(_s) = op.strip_prefix("consts.") {
        // String constants are not lowered to memory yet (CPU data emission
        // deferred); emit a null pointer placeholder.
        let v = builder.ins().iconst(types::I32, 0);
        builder.def_var(d, v);
        return Ok(());
    }
    if op == "id" {
        let v = use_var(builder, get_var(var_of, &var_name(lhs), next_var));
        builder.def_var(d, v);
        return Ok(());
    }

    let a = use_var(builder, get_var(var_of, &var_name(lhs), next_var));
    let b = use_var(builder, get_var(var_of, &var_name(rhs), next_var));
    let v = match op {
        "+" => builder.ins().iadd(a, b),
        "-" => builder.ins().isub(a, b),
        "*" => builder.ins().imul(a, b),
        "/" => builder.ins().sdiv(a, b),
        "%" => builder.ins().srem(a, b),
        "&" => builder.ins().band(a, b),
        "|" => builder.ins().bor(a, b),
        "^" => builder.ins().bxor(a, b),
        "<<" => builder.ins().ishl(a, b),
        ">>" => builder.ins().sshr(a, b),
        "==" => { let z = builder.ins().icmp(IntCC::Equal, a, b); bool_to_i32(builder, z) }
        "!=" => { let z = builder.ins().icmp(IntCC::NotEqual, a, b); bool_to_i32(builder, z) }
        "<" => { let z = builder.ins().icmp(IntCC::SignedLessThan, a, b); bool_to_i32(builder, z) }
        ">" => { let z = builder.ins().icmp(IntCC::SignedGreaterThan, a, b); bool_to_i32(builder, z) }
        "<=" => { let z = builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b); bool_to_i32(builder, z) }
        ">=" => { let z = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, a, b); bool_to_i32(builder, z) }
        "&&" => {
            let za = builder.ins().icmp_imm(IntCC::NotEqual, a, 0);
            let zb = builder.ins().icmp_imm(IntCC::NotEqual, b, 0);
            let z = builder.ins().band(za, zb);
            bool_to_i32(builder, z)
        }
        "||" => {
            let za = builder.ins().icmp_imm(IntCC::NotEqual, a, 0);
            let zb = builder.ins().icmp_imm(IntCC::NotEqual, b, 0);
            let z = builder.ins().bor(za, zb);
            bool_to_i32(builder, z)
        }
        _ => a,
    };
    builder.def_var(d, v);
    Ok(())
}

fn bool_to_i32(
    builder: &mut FunctionBuilder,
    b1: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    builder.ins().uextend(types::I32, b1)
}

fn emit_call(
    dest: &Option<String>,
    callee: &str,
    args: &[String],
    builder: &mut ClifBuilder,
    var_of: &mut HashMap<String, Variable>,
    next_var: &mut u32,
    obj: &mut ObjectModule,
    ids: &HashMap<String, cranelift_module::FuncId>,
) -> Result<()> {
    let callee_name = callee.trim_start_matches('@');
    let callee_name = callee_name.trim_start_matches('%');
    let func_id = ids
        .get(callee_name)
        .cloned()
        .ok_or_else(|| anyhow!("call to undeclared function '{}'", callee_name))?;
    let callee_ref = obj.declare_func_in_func(func_id, &mut builder.func);
    let mut arg_vals = Vec::new();
    for a in args {
        let v = use_var(builder, get_var(var_of, &var_name(a), next_var));
        arg_vals.push(v);
    }
    let call = builder.ins().call(callee_ref, &arg_vals);
    let result = builder.inst_results(call)[0];
    if let Some(d) = dest {
        let dv = get_var(var_of, &var_name(d), next_var);
        builder.def_var(dv, result);
    }
    Ok(())
}

fn emit_terminator(
    term: &Terminator,
    builder: &mut ClifBuilder,
    var_of: &mut HashMap<String, Variable>,
    clif_block: &HashMap<usize, Block>,
    next_var: &mut u32,
) -> Result<()> {
    match term {
        Terminator::Return(Some(reg)) => {
            let v = use_var(builder, get_var(var_of, &var_name(reg), next_var));
            builder.ins().return_(&[v]);
        }
        Terminator::Return(None) => {
            let zero = builder.ins().iconst(types::I32, 0);
            builder.ins().return_(&[zero]);
        }
        Terminator::Jump(t) => {
            let b = *clif_block
                .get(t)
                .ok_or_else(|| anyhow!("jump to missing block {}", t))?;
            builder.ins().jump(b, &[]);
        }
        Terminator::Branch { cond, then_bb, else_bb } => {
            let c = use_var(builder, get_var(var_of, &var_name(cond), next_var));
            let t = *clif_block
                .get(then_bb)
                .ok_or_else(|| anyhow!("branch then missing {}", then_bb))?;
            let e = *clif_block
                .get(else_bb)
                .ok_or_else(|| anyhow!("branch else missing {}", else_bb))?;
            builder.ins().brif(c, t, &[], e, &[]);
        }
        Terminator::Unreachable => {
            builder.ins().trap(cranelift_codegen::ir::TrapCode::UnreachableCodeReached);
        }
    }
    Ok(())
}
