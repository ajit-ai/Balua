Balua — Phase-wise Milestones and Detailed Design Architecture
==================================================================

Local-only plan. No GitHub Pages. Read alongside ``roadmap.rst`` (phase
ledger) and ``language.rst`` (language guide).

Position (``develop@1bc438d``)
------------------------------

Real: lexer/parser/AST, semantic + ``TypeTable``
(``crates/baluac/src/semantic.rs:76``), MIR for ``FnDecl``/``if``/``match``/
loops/calls (``crates/baluac/src/mir.rs:62``), Cranelift ``i32`` executables
via host linker (``crates/baluac/src/backend/cranelift.rs:51``,
``crates/baluac/src/backend/object_emit.rs:63``) — 14 Rust tests green.

Stubs/text only: LLVM/PTX/SPIR-V/HLS/QASM emitters, ``baluald`` fat-binary
(``crates/baluac/src/linker/baluald.rs:22``), ``blpkg`` operations
(``crates/blpkg/src/builder.rs:17``), ``safety/*`` manual flags,
``std/hal/ffi`` sim text, ``tools/*`` README-only, no runtime.

Rule (``spec/NFR-roadmap-v2.md:25``): no phase is done until a **runnable
artifact** executes.

Architecture blueprint (all phases)
------------------------------------

::

  .bl → Lexer → Parser → AST → Semantic(+TypeTable) → MIR → Backend(s) → Link → exe

Contracts to freeze now:

1. **Spans:** ``Expr::span()`` (``crates/baluac/src/ast.rs:277``) returns
   empty defaults, so ``TypeTable`` keys collide. Thread real per-expr
   spans before any type-directed codegen (``mir.rs:67`` admits this).
2. **MIR schema:** ``MirModule/MirFunction/BasicBlock/Instruction/
   Terminator`` (``mir.rs:9``) — version it; ``Phi`` is emitted but never
   consumed (``cranelift.rs:242``): keep or drop, explicitly.
3. **Backend trait:** ``Backend::lower -> String``
   (``backend/mod.rs:16``) forces temp-file round-trips
   (``cranelift.rs:42``). Move to typed artifacts so stubs cannot satisfy
   the trait with comments.
4. **One linker:** ``object_emit::build_executable`` (real, used) vs
   ``BaluaLinker::link`` (prints, dead) — delete one concept.
5. **Modules:** ``UseDecl`` is ignored (``semantic.rs:97``,
   ``mir.rs:112``); ``std/`` is never read by the driver. Decide: sysroot
   + file resolution + symbol table, or single-file language.
6. **Runtime decision:** stay ``i32``-only (document it) or add
   string/array/IO layout — ``std/*.bl`` uses ``Vec``/indexing/
   ``println(fmt)`` the compiler cannot lower.
7. **Safety as MIR passes**, not manual bools (``safety/misra.rs:12``,
   ``safety/wcet.rs:17``).
8. **Manifest:** ``Balua.toml:1`` additive-only versioning
   (``edition="2026"`` → ``"2027"``); ``[hardware]`` additive.

Phase M1 — ``blpkg`` local (NEXT, 3–4 weeks)
--------------------------------------------

Objective: ``blpkg new demo && blpkg build && blpkg run && blpkg test``
works on real code, no ``println!`` stubs.

Design: ``Balua.toml`` serde schema; local path + semver check, no
network; ``builder`` calls ``baluac`` as a library per entry file then
``object_emit``; ``run`` spawns the artifact and reports exit code;
``test`` builds the entry and executes it (sim fallback); ``bom``
minimal CycloneDX JSON.

Tasks (``crates/blpkg/src/``):

- ``builder.rs`` — ``read_manifest`` (``src/main.bl`` convention +
  first-``src/*.bl`` fallback), ``compile_source`` (lex → parse →
  semantic → ``lower_with_types`` → ``build_executable``), error gate on
  ``Severity::Error``; return artifact path.
- ``resolver.rs`` — validate with ``semver::VersionReq::parse`` (real),
  local-only.
- ``bom.rs`` — emit ``license``/``source``/``hash`` when present.
- ``main.rs`` — ``run`` builds then executes; ``test`` builds then
  executes; ``add`` resolves; ``publish``/``fetch`` explicit post-GA
  errors (no fake success).
- Tests: manifest parsing, semver accept/reject, ``bom`` fields (no
  linker in unit tests).

Exit: demo package builds+runs+tests on clean Win11; ``cargo test -p
blpkg`` green. Tag ``phase-3-local``.

Validate::

  cargo build -p blpkg
  cargo test -p blpkg
  blpkg new demo && blpkg build && blpkg run && blpkg test

Risks: registry/network creep — forbid; ``publish``/``add`` stay
explicit post-GA errors.

Phase M2 — Safety-min (3 weeks)
--------------------------------

Objective: ``--safety-profile`` actually rejects.

Design: MIR walkers for heap (``box``/alloc), recursion (call-graph
cycle), unbounded ``loop``-without-``break``; ``#[max_stack(N)]``
frame-sum estimate; ``E_SAFETY_*`` JSON diagnostics; formal verification
stays emit-only.

Tasks: ``safety/{misra.rs,wcet.rs,stack_analysis.rs}``,
``main.rs:58`` enforcement gate before ``-o``; 4–6 fixture tests.

Exit: AUTOSAR fixture fails with code; clean fixture links. Tag
``phase-5-min``.

Phase M3 — DX-min + stabilization (3–4 weeks)
---------------------------------------------

Objective: shippable DX + reproducibility.

Design: one real formatter (``balua-fmt``, lexer-preserving,
idempotent) + real ``balua-test`` runner; freeze ``--help``/JSON schema
(versioned); lexer/parser fuzz 1h zero-crash; repro ``-o`` check;
update ``BLRS`` Ch.2/14, ``NFR-roadmap-v2.md``, local ``docs/``.

Exit: ``cargo build/test`` green, docs render from ``file://``. Tag
``v1.0-ga-rc``.

Phase M4 — GA (1–2 weeks)
--------------------------

Objective: ``main@v1.0-ga`` installable. ``develop → main --no-ff``
(``GIT_WORKFLOW.md:30``), notes (scope vs non-goals), ``balua --version
→ v1.0-ga``, fresh-VM install + ``hello.exe`` demo. Then **stop**.

Post-GA backlog (each: own ``phase/*`` + executable exit + tag)
----------------------------------------------------------------

- **E1 LLVM-real** — ``llvm-sys 221`` (LLVM 22.1.x), O-levels + host machine
  emit
  (``backend/llvm.rs:30``); ``02_cpu_simd_sort`` bench.
- **E2 Parallelism runtime** — ``spawn/chan/select`` + ``Send/Sync``
  executor (``std/sync.bl``, ``std/future.bl``).
- **E3 GPU/NPU** — real ``.cubin`` + ``cudaLaunchKernel``,
  ``spirv-val``, ONNX quant; ``03/04/09`` ≥0.9× cuBLAS.
- **E4 FPGA/quantum** — HLS ``xclbin`` timing-closed, QASM fidelity;
  ``05/06`` on Vitis/Aer.
- **E5 Self-host** — ``selfhost/{ast,mir,semantic}.bl``; ``balua``
  builds ``balua``; ``10_kernel`` boots (needs strings/collections
  first).
- **E6 Ecosystem** — ``blpkg.io`` + provenance/lockfile, full
  ``fmt/lsp/dbg/prof/doc/test/bindgen`` as workspace members.
- **E7 Certification** — AUTOSAR/DO-178C evidence, Frama-C/CBMC;
  ``08_rtos`` 10kHz.

Gate table
----------

.. list-table::
   :header-rows: 1

   * - Gate
     - Depends on
     - Proves
     - Tag
   * - M1
     - M0 done
     - package builds real exe
     - phase-3-local
   * - M2
     - M1
     - unsafe rejected
     - phase-5-min
   * - M3
     - M2
     - DX + repro
     - v1.0-ga-rc
   * - M4
     - M3
     - fresh-VM install
     - v1.0-ga
   * - E*
     - M4
     - one HW/runtime axis each
     - phase-N each
