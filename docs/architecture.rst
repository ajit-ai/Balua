Balua Architecture
==================

Layering (implemented)
----------------------

``.bl`` → Lexer → Parser → AST → Semantic (+ ``TypeTable``) → MIR →
Backend → host link → executable. The driver
(``crates/baluac/src/main.rs:65``) orchestrates the stages inline and gates
``-o`` output on zero ``Error`` diagnostics. Library surface:
``crates/baluac/src/lib.rs:5``. Full stage contract: :doc:`compiler`.

Load-bearing contracts
----------------------

1. **Spans**: most expression spans are defaults (``ast.rs``), so the
   ``TypeTable`` is coverage-grade, not per-expression precise.
2. **MIR schema**: string-typed registers and params; ``Phi`` nodes are
   emitted but never consumed by the backend.
3. **Backend trait**: ``lower -> String`` lets template backends satisfy the
   interface with comments; the real path round-trips through temp object
   files.
4. **Two linkers**: ``object_emit::build_executable`` (real, used) versus
   ``BaluaLinker`` (prints, unused) — one concept must go.
5. **Modules**: top-level ``module name { items }`` parses into
   ``Program.modules``; nested ``module`` is an error; ``use`` imports are
   parsed but never resolved, and ``std/`` is never read.
6. **Runtime**: none exists; generated executables are standalone and
   communicate via exit code only. Strings/collections/I/O have no
   representation yet.
7. **Safety**: MIR walkers over CPU functions (see :doc:`safety`); tiers
   are labels, profiles are policy; hardware regions are skipped.
8. **Manifest**: ``Balua.toml`` is read by ``blpkg`` only; the compiler
   takes source paths and flags (see :doc:`packages`).

Design decisions on record
--------------------------

Hardware-first syntax (``@hw::`` as lexer tokens), ownership without GC,
HM-style inference with tensor-shape errors, C-stable CPU ABI intent,
simulation-fallback HAL, Cranelift-first codegen on a 4GB host budget
(``.cargo/config.toml``), ``develop → main --no-ff`` delivery
(``GIT_WORKFLOW.md``). Phase sequencing and exit rules: :doc:`roadmap`,
:doc:`roadmap-detail`. Known areas needing expert review (layering spans,
type boundaries, module resolution, IR typing, runtime coupling) are
enumerated in the project review record, not redesigned here.
