Balua Compiler
==============

Implementation language: Rust. Library crate ``baluac_lib`` plus driver
bins ``balua`` / ``baluac`` (``crates/baluac/Cargo.toml:7``). Entry:
``crates/baluac/src/main.rs:65``. Module map: ``crates/baluac/src/lib.rs:5``.

Pipeline stages
---------------

Source ``.bl`` → Lexer → Parser → AST → Semantic (+ ``TypeTable``) → MIR →
Backend → host link → executable. Per-file loop with a shared ``mir_all``
accumulator; any ``Error`` diagnostic blocks ``-o`` output.

- **Lexer** (``lexer.rs:57``): UTF-8 scan into ``KEYWORD``, ``IDENTIFIER``,
  ``LITERAL_INT/FLOAT/STR/BOOL``, ``OPERATOR``, ``DELIMITER``, ``COMMENT``,
  ``HARDWARE_DIRECTIVE`` (``@hw::gpu/fpga/npu/quantum/cpu/embedded``),
  ``ANNOTATION`` (``#[...]``), ``EOF``. 47 keywords. Every token carries a
  ``Span`` (``diagnostics.rs:5``).
- **Parser** (``parser.rs:14``): recursive descent with ``synchronize()``
  recovery. Top-level ``module name { items }`` blocks parse into
  ``Program.modules``; a nested ``module`` is a loud error (silently
  swallowing the rest of the file was fixed in M2). Supports
  ``fn``/``let``/``const``/``if``/``match``/``for``/``while``/``loop``/
  ``return``/``break``/``continue``/``spawn``/``chan``/``send``/``recv``/
  ``select``/``unsafe``/``extern``. Binary operators are right-recursive
  with no precedence table; there is no parenthesis-grouping rule.
- **AST** (``ast.rs:9``): ``Program``/``Module``/14 ``Item`` variants;
  ``FnDecl`` carries generics, where-clause, params, return type,
  hardware annotation, async/extern flags, visibility, safety tier, body,
  raw ``attrs``, span. Note: most ``Expr::span()`` values are default
  empties, so ``TypeTable`` keys are coarse (documented in ``mir.rs:67``).
- **Semantic** (``semantic.rs:76``): ``analyze()`` returns diagnostics plus
  ``TypeTable``. Scoped environments, Hindley-Milner-style ``unify``
  (``types/inference.rs:34`` — equality plus tensor-shape check, no
  occurs-check), borrow/move recording (``types/borrow_checker.rs:23``),
  hardware-context validation. ``UseDecl`` imports are not resolved.
  Codes: ``E_TYPE_MISMATCH``, ``E_USE_AFTER_MOVE``,
  ``E_BORROW_CONFLICT``, ``E_HW_*``, ``E_KERNEL_TARGET``,
  ``E_CIRCUIT_TARGET``, ``W_TRUSTED_FUNCTION``, ``W_UNKNOWN_ATTR``.
- **MIR** (``mir.rs:62``): ``MirModule``/``MirFunction`` (name, hardware,
  string-typed params, ``attrs``, span) of ``BasicBlock``s with
  ``Alloca``/``Load``/``Store``/``BinOp``/``Call``/``HwIntrinsic``/
  ``MoveToDevice``/``Phi`` and ``Return``/``Jump``/``Branch``/
  ``Unreachable`` terminators. ``lower``/``lower_with_types``/``to_json``
  (``--emit-mir``). Real lowering covers ``FnDecl`` bodies (``if``/``match``
  chains, ``for`` ranges, ``while``/``loop``/``break``/``continue``,
  calls); ``KernelDecl``/``CircuitDecl`` lower to a single
  ``HwIntrinsic`` + ``Unreachable``; strings lower to null placeholders;
  casts are pass-through; calls require ``Ident`` callees.
- **Optimizations**: none exist as passes. ``--release`` selects Cranelift
  ``opt_level`` 2 (``speed``) instead of 0; LLVM LTO/PGO/BOLT are struct
  fields rendered as comments.

Backends
--------

Dispatch: ``select_backend`` (``backend/mod.rs:21``).

- **Cranelift (real)**: ``backend/cranelift.rs:51`` lowers MIR to CLIF via
  ``cranelift-* 0.110`` (all values ``i32``; comparisons widened via
  ``uextend``) and emits a relocatable object; ``backend/object_emit.rs:63``
  links it with the host toolchain (``cc``/``gcc``/``clang``/``cl`` probe)
  into a runnable executable. Unknown callees are link-time errors.
  Hardware intrinsics and device moves lower to zero-fill.
- **LLVM (dual-mode)**: ``backend/llvm.rs:20`` emits string-builder IR by
  default; with ``--features llvm`` (llvm-sys 191, LLVM 19.1.x via
  ``LLVM_SYS_191_PREFIX``) it builds a real in-memory module (alloca model:
  every virtual register gets an entry-block slot; integer ``i32`` domain
  like Cranelift) and can emit native objects (``compile_llvm_to_object``)
  for ``-o``. ``--cpu-backend`` selects ``cranelift`` (default, works
  everywhere) or ``llvm`` (fail-closed guidance without the feature).
  ``lto``/``pgo``/``bolt`` fields are accepted but unimplemented; machine
  emission is host-targeted (``--target`` triple is recorded, cross-target
  objects not validated).
- **PTX / SPIR-V / HLS / OpenQASM / MLIR (templates)**: fixed headers
  (PTX 8.0 + ``wmma`` strings, SPIR-V 1.6 opcodes, HLS pragmas + Tcl,
  QASM gate list, ``balua`` dialect skeleton) with ``MIR: {:?}`` comments.
  No validator or execution is wired.
- **Linker**: ``-o`` uses ``object_emit`` directly. ``linker/baluald.rs:22``
  (fat-binary stitching) only prints and is never called by the driver.

CLI
---

``balua [files...] [--emit-mir] [--emit-llvm] [--emit-clif]
[--backend HW] [--cpu-backend llvm|cranelift] [--release] [-o EXE]
[--keep-object] [--target TRIPLE] [--json-diagnostics]
[--safety-profile P] [--safety-stack-limit N] [--verbose]``.
``--emit-*`` prints to stdout; diagnostics honor ``--json-diagnostics``;
``--verbose`` adds per-stage timings, type-table size, and the safety
stack report. ``main()`` returning ``i32`` becomes the process exit code.
The flag set above is frozen by ``crates/baluac/tests/cli_tests.rs``.

Diagnostics JSON schema (frozen)
---------------------------------

``Diagnostic.to_json()`` keys: ``severity`` (lowercase: ``error``,
``warning``, ``info``, ``hint``), ``code``, ``message``, ``span``
(``file``, ``line``, ``col``, ``end_line``, ``end_col``), ``hint``,
``hardware_context``. Compile profile (``--verbose`` JSON) keys:
``events`` (``kind``, ``duration_ms``, ``detail``), ``total_ms``; note
``EventKind`` renders capitalized (``Lex``, ``Parse``, ``Semantic``,
``Codegen``, ``Link``, ``Opt``). Covered by goldens in
``crates/baluac/src/diagnostics.rs``.
