Balua Roadmap — Current Phase and Future Scope
================================================

Two ledgers exist. ``spec/NFR-roadmap-v2.md:1`` is the grounded source of
truth (verified by executing tests). ``spec/NFR-roadmap.md:1`` is the
legacy full plan (v0.9.0 → v1.0.0). Both are summarised here.

Status legend (v2)
------------------

- ``VERIFIED`` — real logic, wired into pipeline, covered by an executing test.
- ``WIRED, SHALLOW`` — runs but thin.
- ``ORPHANED`` — real logic exists but nothing calls it.
- ``STUB`` — signatures / string templates only.

See ``spec/NFR-roadmap-v2.md:18``.

Grounded ledger (2026-09-03 audit + 2026-09-21 git)
---------------------------------------------------

.. list-table::
   :header-rows: 1

   * - Phase
     - Status
     - Verified by / evidence
   * - Phase 1 — Wire semantic to type/borrow
     - VERIFIED 2026-09-03
     - ``crates/baluac/tests/semantic_tests.rs`` (8 tests pass); ``cc7d74c``
   * - Phase 2 — One real CPU backend (Cranelift) end to end
     - IN PROGRESS (working now)
     - ``77a2af1`` + ``3340e6c`` merge ``phase/2-cranelift -> develop``
   * - Phase 3 — ``blpkg`` builds/runs projects
     - NOT STARTED
     - stub: ``resolver``, ``registry``, ``builder``, ``bom``
   * - Phase 4 — Self-host bootstrap
     - NOT STARTED
     - ``selfhost/*.bl`` stub/mirror only
   * - Phase 5 — Safety that analyzes
     - NOT STARTED
     - ``safety/*.rs`` stubs
   * - Phase 6+ — Hardware targets (GPU/FPGA/NPU/Quantum)
     - NOT STARTED
     - ``backend/`` (except ``cranelift.rs``), ``std/hal/*.bl``, ``std/ffi/*.bl`` stubs

Audit note: real and working are ``lexer.rs``, ``parser.rs``, ``ast.rs``,
basic ``mir.rs``, ``diagnostics.rs``. Orphaned-until-Phase-1 (now wired):
``types/inference.rs``, ``types/borrow_checker.rs``.
See ``spec/NFR-roadmap-v2.md:29``.

Working now — Phase 2 detail
----------------------------

Goal (``spec/NFR-roadmap-v2.md:51``):

::

  balua hello.bl -o hello.exe && ./hello.exe

produces real output / exit code, using Phase-1 ``TypeTable`` to source
CLIF types.

Files in scope:

- ``crates/baluac/src/backend/cranelift.rs:1``
- ``crates/baluac/src/backend/object_emit.rs:1``
- ``crates/baluac/src/mir.rs:1``
- ``crates/baluac/src/backend/mod.rs:21``
- ``crates/baluac/src/main.rs:144`` (``build_executable`` path)
- ``crates/baluac/tests/execution.rs:1``
- ``hello.bl:1``, ``keyword_test.bl:1``, ``examples/01b_*.bl`` … ``06a_*.bl``

Branches/tags: ``develop`` (active), ``phase/2-cranelift``,
``phase/10-selfhost-update``; latest grounded tag ``phase-1-semantic``.
``git log`` head: ``3340e6c`` merge Phase 2, ``77a2af1`` Phase-2 backend,
``cbe93af`` / ``cc7d74c`` Phase 1.

Exit criteria: runnable, observable artifact — not “compiles”.
No ``phase-2`` tag until ``hello.exe`` runs and tests pass
(``spec/NFR-roadmap-v2.md:69``).

Future scope (grounded order)
-----------------------------

1. Phase 3 — Ecosystem (local build/run): ``blpkg`` resolver → builder →
   runner; ``BPM build/run/test`` on real projects.
2. Phase 4 — Scalability / self-host: ``selfhost/ast.bl``, ``mir.bl``,
   ``semantic.bl``, ``lexer.bl``, ``parser.bl``; ``balua`` builds ``balua``.
3. Phase 5 — Safety: WCET/stack/certification that actually analyzes
   (MISRA, AUTOSAR/DO-178C, Frama-C/CBMC).
4. Phase 6+ — Hardware: GPU (PTX/CUBIN), SPIR-V, NPU ONNX, FPGA HLS
   ``xclbin``, Quantum QASM fidelity — each with hardware-in-loop validation.

NFR-to-phase (v2): correctness→1 (done), runnable CPU→2 (now),
ecosystem→3, self-host→4, safety→5, hardware→6+.
See ``spec/NFR-roadmap-v2.md:58``.

Legacy full plan (for reference)
---------------------------------

``spec/NFR-roadmap.md:18`` claimed DONE to ``v1.0.0`` on ``main@3aba7e2``:

- Phase 0 scaffold ``v0.1.0`` — DONE
- Phase 1 backend hardening ``phase-1`` — DONE
- Phase 2a keywords (``const/match/for/while/loop/break/continue/where/pub/priv/cast``) — DONE
- Phase 2b LLVM string/llvm-sys split, LTO/PGO/BOLT — DONE
- Phase 3 parallelism (``spawn/chan/send/recv/select``, ``Send``/``Sync``) — DONE (stub per v2)
- Phase 4 security/safety tiers + SBOM — DONE (stub per v2)
- Phase 5 GPU/NPU wmma + ``@hw::npu`` ``v0.6.0`` — DONE (stub per v2)
- Phase 6 FPGA/Quantum HLS DATAFLOW + QASM ``v0.7.0`` — DONE (stub per v2)
- Phase 7 DX/Observability ``v0.8.0`` — DONE (stub per v2)
- Phase 8 self-host lexer/parser ``v0.9.0`` — DONE (mirror only per v2)
- Phase 9 ecosystem ``v1.0.0`` — DONE on legacy ledger (docs/registry)

Treat legacy DONE as “scaffolded”, not verified. Use v2 for execution.

Git workflow per phase
----------------------

See ``GIT_WORKFLOW.md:15``:

::

  git checkout develop
  git checkout -b phase/N-<name>
  # ... implement + cargo build/test ...
  git commit -m "feat(phase-N): <name>"
  git push origin develop
  git checkout main
  git merge --no-ff develop -m "merge: Phase-N → main (Balua v0.N.0)"
  git tag -a phase-N -m "Phase N complete"
  git push origin --tags

Next action: finish Phase 2 linked executable, cut ``phase-2`` tag only
after exit criteria execute green.
