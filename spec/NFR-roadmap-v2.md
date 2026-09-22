# Balua Implementation Roadmap v2 — Grounded in Audited Reality
> Companion: `ARCHITECTURE.md`. This document's phase ledger is the source of truth for
> what is genuinely done (verified by executing tests) versus what merely compiles.

## Status ledger

| Phase | Status | Exit criteria met | Verified by |
|-------|--------|-------------------|-------------|
| Phase 1 — Wire semantic analysis to type/borrow checkers | ✅ VERIFIED — 2026-09-03 | Type mismatch → E_TYPE_MISMATCH; use-after-move → E_USE_AFTER_MOVE; borrow conflict → E_BORROW_CONFLICT; valid programs produce zero errors; TypeTable populated | `crates/baluac/tests/semantic_tests.rs` (8 tests, all pass) |
| Phase 2 — One real CPU backend (Cranelift) end to end | 🔴 NOT STARTED | — | — |
| Phase 3 — `blpkg` builds/runs real projects | 🔴 NOT STARTED | — | — |
| Phase 4 — Self-hosting bootstrap | 🔴 NOT STARTED | — | — |
| Phase 5 — Safety analysis that analyzes | 🔴 NOT STARTED | — | — |
| Phase 6+ — Hardware targets | 🔴 NOT STARTED | — | — |

## How to read status

| Tag | Meaning |
|---|---|
| ✅ VERIFIED | Has real logic, wired into the pipeline, covered by a test that executes real behavior and passes. |
| 🟡 WIRED, SHALLOW | Runs as part of the pipeline and does something real, but logic is thin. |
| ⚪ ORPHANED | Real logic exists but nothing calls it. |
| 🔴 STUB | Signatures/string templates only, no behavior. |

No phase is complete until its exit criteria produce a **runnable, observable artifact**.

## Audit summary (2026-09-03)

**Real and working:** `lexer.rs`, `parser.rs`, `ast.rs`, basic `mir.rs`, `diagnostics.rs`.
**Orphaned until Phase 1 (now wired):** `types/inference.rs`, `types/borrow_checker.rs`.
**Stubs (Phase 6+ scope, do NOT touch during 1–3):** every file under `backend/` except
`cranelift.rs`, `linker/baluald.rs`, `safety/*.rs`, `blpkg::{resolver,registry,builder,bom}`,
`std/hal/*.bl`, `std/ffi/*.bl`, `selfhost/*.bl`.

## Phase 1 — Wire semantic analysis to the type/borrow checkers (DONE)

**Files touched:**
- `crates/baluac/src/semantic.rs` — added `InferenceEngine`, `BorrowChecker`, `TypeEnv`,
  `hw_context`; `analyze` now returns `(Vec<Diagnostic>, TypeTable)`; `analyze_expr`/`analyze_block`
  unify every expression and record borrows/moves.
- `crates/baluac/src/types/borrow_checker.rs` — diagnostics now carry `E_BORROW_CONFLICT` /
  `E_USE_AFTER_MOVE` codes.
- `crates/baluac/src/diagnostics.rs` — added `Span: Hash+Eq+PartialEq` derives and `with_code`.
- `crates/baluac/src/ast.rs` — added `Expr::span()` helper for TypeTable keying.
- `crates/baluac/src/main.rs` — consumes new `(Vec<Diagnostic>, TypeTable)` return.
- `crates/baluac/tests/semantic_tests.rs` (NEW) — 8 exit-criteria tests, all pass.

**Not touched (per phase scope):** anything under `backend/`, `linker/`, `safety/`, `std/`,
`selfhost/`, `blpkg/`.

## Phase 2 — One real backend (Cranelift) end to end (NEXT)

Goal: `balua hello.bl -o hello.exe && ./hello.exe` produces real output/exit code.
Dependencies on Phase 1's `TypeTable` to source CLIF types.

## NFR-to-Phase reference (v2)

| NFR | Phase | Status today |
|---|---|---|
| Compilation correctness (types/borrow) | 1 | ✅ VERIFIED |
| Performance / runnable CPU output | 2 | Not started |
| Ecosystem (local build/run) | 3 | Not started |
| Scalability / self-host | 4 | Not started |
| Safety (WCET/stack/certification) | 5 | Not started |
| Hardware (GPU/FPGA/NPU/Quantum) | 6+ | Not started |

## Git workflow

Keep `GIT_WORKFLOW.md` as-is. A `phase-N` tag is cut only once that phase's exit criteria
(an executed, verified artifact) are met — not when code compiles.

## M3 DX note (develop, delivered tooling)

DX work delivered outside the language-phase ledger above: `balua-fmt`
(`--check`/`--write`, idempotent, token-preserving, all in-repo `.bl`
clean), `balua-test` (exit-0 convention, `--bench`, timeouts), frozen CLI
flags and diagnostics JSON schema (regression-tested), fixed-seed fuzz
smoke over token soup plus the full `.bl` corpus (no-panic contract),
byte-identical `.o` objects and GNU-linked executables
(`-Wl,--no-insert-timestamp -Wl,--build-id=none`; MSVC unverified), and CI
jobs (`build`, `fmt`, `test-runner`, `blpkg`, strict Sphinx docs). This
note does not change the phase ledger; language milestones still gate on
runnable artifacts per phase.
