# Git Workflow — Balua Phased Delivery

> Branching model: `main` (protected, production) ← `develop` (integration) ← `phase/*` / `feature/*`

## Branches

| Branch | Purpose | Source |
|--------|---------|--------|
| `main` | Stable, release-ready. All phased deliverables merged here after review. | `develop` via `--no-ff` merge |
| `develop` | Integration branch. Each Phase (1–4) is committed here first, tested (`cargo test`, `blpkg build`), then merged to `main`. | `main` |
| `phase/1-foundation` … `phase/4-production` | Optional per-phase feature branches. Merge into `develop` via PR. | `develop` |

## Per-Phase Commit → Merge to `main`

For **each Phase** (Section 13 Implementation Roadmap):

```powershell
# 1. Work on develop (or phase/X branch)
git checkout develop
git pull origin develop

# 2. Implement Phase deliverables, then commit to develop
git add <phase files>
git commit -m "feat(phase-N): <deliverable> — see spec Section X"

# 3. Push develop
git push origin develop

# 4. Merge develop → main (no fast-forward, preserves phase history)
git checkout main
git pull origin main
git merge --no-ff develop -m "merge: Phase-N → main (Balua v0.N.0)"
git push origin main

# 5. Tag release
git tag -a v0.N.0 -m "Balua Phase N complete"
git push origin v0.N.0
```

### Phases (from spec)

- **Phase 1 — Foundation (1–6 mo):** Lexer/parser/AST/MIR, LLVM x86_64/aarch64, borrow checker, `std::core/mem/alloc`, `blpkg build/test`, `examples/01-02` — `spec/BLRS-v1.0.md:1`, `crates/baluac/src/lexer.rs:1`
- **Phase 2 — GPU & NPU (7–12 mo):** PTX `crates/baluac/src/backend/ptx.rs:1` + SPIR-V `crates/baluac/src/backend/spirv.rs:1`, `@hw::gpu`, `std::hal::gpu/npu`, `std::tensor` — `examples/03-04`
- **Phase 3 — FPGA & Quantum (13–18 mo):** HLS `crates/baluac/src/backend/hls.rs:1` + OpenQASM `crates/baluac/src/backend/openqasm.rs:1`, `std::hal::fpga/quantum`, `tools/balua-fmt/lsp/dbg` — `examples/05-06`
- **Phase 4 — Safety & Production (19–24 mo):** MISRA `crates/baluac/src/safety/misra.rs:1`, WCET `crates/baluac/src/safety/wcet.rs:1`, stack `crates/baluac/src/safety/stack_analysis.rs:1`, formal `crates/baluac/src/safety/formal_verification.rs:1`, `no_std`, self-hosting, `blpkg.io`, `examples/07-10`

## Protection

- `main` requires PR from `develop`, `cargo build` + `cargo test` passing, linear history via merge commits.
- `develop` is the source of truth for daily work; never commit directly to `main`.

## Current Status

- Initial scaffold (Phases 1–4 stubs) committed to `develop` and merged to `main` as `v0.1.0` — see `git log --graph --oneline --all`.

## Commands reference

```powershell
git branch -a
git log --oneline --graph --all -20
git diff develop..main --stat
```
