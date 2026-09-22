# Balua Language Reference Specification (BLRS) v1.0

> Version 1.0 | Language Design Authority | 2026
> One Language. Every Silicon. — CPU · GPU · NPU · FPGA · Quantum · Embedded

This document is the authoritative BLRS. Chapters 1–15 + Appendices A–D correspond to Section 12.1 of the Master Design Spec. Full ~200-page spec is condensed here; EBNF grammar lives in `spec/grammar/balua.ebnf`, ABI in `spec/abi/balua-abi-v1.md`.

---

## Chapter 1 — Introduction & Design Goals
Balua is a statically-typed, compiled system language for heterogeneous compute. Philosophy: zero-cost abstractions, hardware-first, memory safety without GC, unified toolchain, determinism, interoperability (C/C++/Rust/CUDA/OpenCL/HLS/VHDL/Verilog/Q#).

## Chapter 2 — Lexical Structure
- Source encoding: UTF-8, Unicode identifiers.
- Tokens: KEYWORD, IDENTIFIER, LITERAL_INT/FLOAT/STR/BOOL, OPERATOR, DELIMITER, COMMENT, HARDWARE_DIRECTIVE (`@hw::`), ANNOTATION (`#[...]`, `#pragma`), EOF.
- Keywords ~40 (`fn`, `let`, `kernel`, `circuit`, `tensor`, `qubit`, ...). See Appendix A.
- Formatting (M3): `balua-fmt` normalizes whitespace/indentation from the token stream only (LF endings, single blank lines kept, `};` joined); it never reorders tokens and is idempotent. All in-repo `.bl` files are formatter-clean (`*.bl text eol=lf`).

## Chapter 3 — Types
- Primitives: `i8`..`i128`, `u8`..`u128`, `f16`/`f32`/`f64`/`f128`, `fp16`/`bf16`/`tf32`, `bool`, `char`, `qubit[N]`, `*mut T`/`*const T`/`&T`/`&mut T`, `v128<T>`/`v256<T>`/`v512<T>`.
- Composites: struct, enum, trait, arrays, slices, `Tensor<T,Shape>`, `stream<T>`.
- Hardware-parameterised: `GpuTensor<T,Shape,Backend: GpuBackend>`, `FpgaBuf<T,Depth,Clock>`.
- Dependent: `fn matmul<const M,K,N>(A: Matrix<f32,M,K>, B: Matrix<f32,K,N>) -> Matrix<f32,M,N>` — dimension mismatch is compile-time error.

## Chapter 4 — Expressions & Operators
- Precedence: Appendix B. Assignment `=`, comparison `== != < > <= >=`, arithmetic `+ - * / %`, bitwise, `&& ||`, range `..`.
- Effects: `fn foo() -> T / Pure | IO | HardwareIO | Unsafe` — callee effects must be subsumed by caller.

## Chapter 5 — Statements & Control Flow
`let`, `if`/`else`, `for`/`while`/`loop`, `match`, `return`, `spawn`, `chan`, `select!`, `unsafe`.

## Chapter 6 — Functions & Closures
`fn`, `async fn`, `extern "C" fn`, `kernel fn`, `circuit fn`, `tensor fn`. Closures capture by borrow/move.

## Chapter 7 — Modules & Packages
`module`, `use hw::cuda::Grid;` manifest `Balua.toml` (cargo-like with `[hardware]` section).

## Chapter 8 — Memory Model & Ownership
Single owner, move semantics, `&T`/`&mut T`, `box<T>::new`, lifetimes `'a`, hardware transfer `x.move_to_device()` invalidates `x`. Cross-device lifetimes checked.

## Chapter 9 — Hardware Abstractions (@hw::)
`@hw::gpu(backend=cuda, sm=90)`, `@hw::fpga(target=ultrascale_plus, clock_mhz=250)`, `@hw::npu(backend=onnx)`, `@hw::quantum(backend=openqasm3, qubits=5)`, `@hw::cpu`, `@hw::embedded`. Backend selection per annotated region; `baluald` fat-binary stitching.

## Chapter 10 — Concurrency & Async
`spawn fn`, `chan<T>`, `select!`, `Future`/`async`/`await`, `Mutex`/`RwLock`/`Condvar` per `std::sync`.

## Chapter 11 — FFI & Interoperability
`extern "C"`, `extern "C++"`, `extern "rust"`, `extern "cuda"`, `extern "opencl"`, Python embedding, VHDL/Verilog, WASM target `wasm32-unknown-unknown`. Tool `balua-bindgen` emits `.blh`.

## Chapter 12 — Safety & Real-Time Profiles
`baluac --safety-profile=AUTOSAR_CPP14|DO-178C|IEC61508` disables heap/recursion/unbounded loops. `#[max_stack(N)]`, `#[wcet_cycles(N)]`, `#[interrupt_handler(IRQ=N)]`, `#[requires]/[ensures]`, `#![no_panic]`. WCET/stack verified, Frama-C/CBMC emitted.

## Chapter 13 — Standard Library Overview
`std::core/ mem/ alloc/ collections/ string/ io/ fs/ net/ thread/ sync/ atomic/ time/ math/ simd/ tensor/ hal/ ffi/ panic/ test/ fmt/ iter/ future/ log` — all `no_std` compatible with `no_std` feature.

## Chapter 14 — Compiler & Toolchain Reference
`baluac` stages: Lexer → Parser (recursive-descent, typed AST) → Semantic (HM + ownership) → MIR (SSA, hw-annotated BBs) → LLVM/PTX/SPIR-V/HLS/QASM/MLIR → `baluald`. `blpkg` commands: `new/build/run/test/bench/add/publish/doc/fmt/lint/cross`. Tools: `balua-fmt/lsp/dbg/prof/doc/test/bindgen`.

M3 tooling guarantees (implemented): `balua-fmt` (`--check`/`--write`/stdout, directory-aware, idempotent, token-preserving); `balua-test` (compile-link-run harness, pass iff exit 0, `--bench[=N]` mean timing, per-run timeout); frozen CLI flags (see `crates/baluac/tests/cli_tests.rs`); diagnostics JSON schema frozen (`severity/code/message/span/file/line/col/end_line/end_col/hint/hardware_context`; profile `events/kind/duration_ms/detail/total_ms`; `EventKind` renders `Lex/Parse/Semantic/...` capitalized); fuzz smoke (fixed-seed token soup + full `.bl` corpus, no-panic contract); reproducibility (`.o` byte-identical; linked `.exe` byte-identical via `-Wl,--no-insert-timestamp -Wl,--build-id=none` on GNU toolchains, MSVC unverified); CI (`build`, `fmt --check`, `test-runner`, `blpkg`, strict Sphinx docs).

## Chapter 15 — Grammar (full EBNF)
See `spec/grammar/balua.ebnf` — LL(k) unambiguous.

## Appendix A — Keyword List
`fn let mut const struct enum trait impl mod module use if else for while loop match return break continue unsafe async await spawn chan select extern type where pub self Self super crate box move in as is kernel circuit tensor qubit true false` + annotations.

## Appendix B — Operator Precedence (highest→lowest)
`* / %` → `+ -` → `<< >>` → `&` → `^` → `|` → `== != < > <= >=` → `&&` → `||` → `= +=` → `..` → `-> => ::`.

## Appendix C — Hardware Target Reference
CPU `x86_64/aarch64/riscv64/arm-none-eabi`; GPU SM70–SM90 CUDA PTX + SPIR-V 1.6; NPU Qualcomm HTP/Apple ANE/Intel NPU/Samsung/MediaTek; FPGA UltraScale+/Versal/Agilex; QPU IBM/Azure/IonQ/Rigetti.

## Appendix D — ABI
See `spec/abi/balua-abi-v1.md`.
