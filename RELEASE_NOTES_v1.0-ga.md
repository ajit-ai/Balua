# Balua v1.0-ga — Release Notes (CPU-first GA)

Version: `balua v1.0-ga` (`balua --version`). Workspace stays `0.1.0`;
`blpkg`/`BPM` report `0.1.0`. Rehearsed with release-built binaries on
Windows 11 (MSYS2 UCRT64 `cc` linker present).

## Install

Prerequisites: Rust stable toolchain plus a host C linker (`cc`, `gcc`,
`clang`, or `cl` on PATH).

```powershell
cargo install --path crates/baluac   # balua.exe, baluac.exe
cargo install --path crates/blpkg    # blpkg.exe, BPM.exe
balua --version                      # balua v1.0-ga
```

## Demo transcript (fresh directory, installed binaries only)

```powershell
blpkg new demo
# Created Balua package 'demo'
blpkg build
# Build succeeded: src/main.bl -> target/debug\demo.exe
blpkg run
# Ran target/debug\demo.exe — exit 0
blpkg test
# test(sim): entry target/debug\demo.exe — exit 0 (simulation fallback, no hardware needed)
balua src/main.bl -o demo-safe.exe --safety-profile=AUTOSAR_CPP14
# safety profile AUTOSAR_CPP14: 1 functions checked (0 hardware-targeted skipped), 0 violations
# balua: produced executable demo-safe.exe
```

`blpkg new` accepts absolute paths (the package name is the final path
component; passing a path verbatim used to break `Balua.toml` parsing —
fixed in this release).

## In scope

- Integer-focused CPU subset: `fn` calls, `let`/`mut`, `if`/`match`,
  `for`/`while`/`loop`/`break`/`continue`, arithmetic/comparison/bitwise
  operators, `as` casts (pass-through).
- `balua-fmt` (`--check`/`--write`, idempotent, token-preserving; repo
  `.bl` all clean) and `balua-test` (pass iff exit 0, `--bench`, timeouts).
- Frozen CLI flags and diagnostics/profile JSON schemas (regression tests).
- MIR safety enforcement: heap/recursion/unbounded-loop rejection,
  `#[max_stack]`/`#[wcet_cycles]`, `--safety-stack-limit`, unknown-profile
  failure; WCET model intentionally structural.
- Deterministic builds: byte-identical `.o` and GNU-linked executables.
- Fixed-seed fuzz smoke plus full `.bl` corpus, no-panic contract.

## Non-goals (post-GA)

Real GPU/NPU/FPGA/quantum execution, full LLVM codegen, `blpkg.io`
registry, self-hosting compiler, LSP/debugger/profiler, certification
evidence, GitHub Pages deployment claims beyond the Sphinx build.

## Verification (this release)

`lexer` 2, `parser` 2, `semantic` 8, `safety` 13, `cli` 3, `fuzz` 2,
`lib` 6, `execution` 2 (17 end-to-end cases), `blpkg` 8×2,
`balua-fmt` 6, `balua-test` 4 — all green; `sphinx-build -b html -W`
clean (16 pages).

## Known limitations

- 18 of 21 `examples/*.bl` report diagnostics (measured inventory in
  `docs/examples.rst`); only `hello.bl` is linked-and-executed in-tree.
- MSVC `cl` link path keeps prior behavior (deterministic flags cover
  GNU toolchains only).
- No Balua runtime exists: programs communicate via exit code; strings,
  collections, and concurrency have no executable lowering.
