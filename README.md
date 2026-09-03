# Balua — System-Level Programming Language

> **One Language. Every Silicon.** CPU · GPU · NPU · FPGA · Quantum · Embedded
>
> Version 1.0 | Language Design Authority | 2026

Balua is a next-generation, open-source, statically-typed, compiled system language — the universal substrate for heterogeneous compute. Zero-cost abstractions, hardware-first primitives, memory safety without GC, unified toolchain.

This repo is a from-scratch implementation of the [Balua Master Design & System Requirements Specification](spec/BLRS-v1.0.md) — 13 sections covering compiler, HAL, type system, stdlib, package manager, toolchain, FFI, safety, and reference programs.

## Quick Start

```powershell
# Build compiler + package manager
cargo build

# Compile a Balua file (example)
cargo run -p baluac -- examples/01_hello_hardware.bl --emit-mir
cargo run -p baluac -- examples/03_gpu_matrix_multiply.bl --emit-llvm --target x86_64 --json-diagnostics

# Package manager
cargo run -p blpkg -- new my_app
cargo run -p blpkg -- build
cargo run -p blpkg -- build --target gpu:cuda:sm90
cargo run -p blpkg -- build --target fpga:xilinx:ultrascale_plus
cargo run -p blpkg -- build --target quantum:ibm:fake_manila
```

## Repository Layout

```
Cargo.toml                  # workspace (baluac + blpkg)
Balua.toml                  # example Balua package manifest
crates/baluac/              # compiler — lexer/parser/AST/semantic/MIR/backends/linker/safety
  src/lexer.rs, parser.rs, ast.rs, semantic.rs, mir.rs, diagnostics.rs
  src/backend/{llvm,ptx,spirv,hls,openqasm,mlir_dialect}.rs
  src/linker/baluald.rs
  src/types/{inference, borrow_checker, lifetime, hardware_types, effect_system}.rs
  src/safety/{misra,wcet,stack_analysis,formal_verification}.rs
crates/blpkg/               # package manager — resolver (PubGrub), builder, registry
std/                        # standard library — std::core/mem/alloc/tensor/hal/ffi ...
  hal/{cpu,gpu,npu,fpga,quantum,embedded,memory}.bl
spec/                       # language spec — BLRS + EBNF + ABI
  BLRS-v1.0.md
  grammar/balua.ebnf
  abi/balua-abi-v1.md
examples/01_..10_*.bl        # 10 reference programs (hello → OS kernel)
tools/{balua-fmt,lsp,dbg,prof,doc,test,bindgen}/  # toolchain DX
tests/hal/sim_tests.bl      # HAL simulation-fallback tests
```

## Compiler Pipeline (Section 2)

```
.bl source → Lexer (Unicode, @hw:: tokens) → Parser (recursive-descent, typed AST) 
→ Semantic (HM inference + ownership/borrow + cross-device lifetimes + hw validation)
→ MIR (SSA, hw-annotated BBs) → Backends → baluald (fat-binary)
```

- **LLVM** — x86_64/aarch64/riscv64/arm-none-eabi, LTO/PGO/BOLT
- **PTX** — SM 70–90, host stubs
- **SPIR-V 1.6** — Vulkan compute / OpenCL 3.0
- **HLS C** — Vitis (UltraScale+/Versal/Agilex) + Tcl/xdc
- **OpenQASM 3.0 / Q#** — IBM/Azure/IonQ/Rigetti
- **MLIR** — custom `balua` dialect hub

## Hardware Abstraction Layer (Section 4)

`std::hal::{cpu,gpu,npu,fpga,quantum,embedded,memory}` — every function has a simulation fallback for `#[test(sim=true)]` on hosts without hardware.

## Roadmap (Section 13)

### Completed — v0.9.0 (2026)
- **Phase 1** (1–6 mo): CPU-only Balua — lexer/parser/LLVM/ownership, `std::core/mem/alloc`, `blpkg build/test`
- **Phase 2a** (7–8 mo): Keywords — `const`, `match`, `for/while/loop`, `break/continue`, `where`, `pub/priv`, `cast(as)`
- **Phase 2b** (8–9 mo): LLVM backend — string-builder IR, `--features llvm` for llvm-sys 18, LTO/PGO/BOLT hooks
- **Phase 3** (9–10 mo): Parallelism runtime — `spawn`, `chan`, `send`, `recv`, `select!`, `Send`/`Sync`
- **Phase 4** (10–11 mo): Security/safety — `safe`/`unsafe`/`trusted` tiers, SBOM (`Bom.toml`), supply chain
- **Phase 5** (11–12 mo): GPU/NPU execution — wmma PTX instructions, `@hw::npu`, v0.6.0
- **Phase 6** (12–13 mo): FPGA/Quantum — HLS `#pragma HLS DATAFLOW`, OpenQASM 3.0 gates (h/cx/t/s/x/y/z), v0.7.0
- **Phase 7** (13–14 mo): DX/Observability — compile events, `--verbose`, profile JSON, v0.8.0
- **Phase 8** (14–15 mo): Self-host — `selfhost/{lexer,parser}.bl` mirror current compiler, v0.9.0

### Planned — v1.0.0 (15–18 mo)
- **Phase 9**: Ecosystem — `balua-fmt/lsp/dbg/prof/doc/test/bindgen` production, `blpkg.io` registry, docs, v1.0.0

## Spec & Grammar

- Spec: [`spec/BLRS-v1.0.md`](spec/BLRS-v1.0.md) (15 chapters + Appendices A–D)
- EBNF: [`spec/grammar/balua.ebnf`](spec/grammar/balua.ebnf)
- ABI:  [`spec/abi/balua-abi-v1.md`](spec/abi/balua-abi-v1.md)
- File ext: `.bl`, headers `.blh`, manifest `Balua.toml`, compiler `baluac`, pm `blpkg`, REPL `balua`

## Contributing

See GitHub Issues. By design: `baluac` in Rust, self-hostable in Balua within 3 major versions, parallel compilation, JSON diagnostics for IDEs.

## License

MIT OR Apache-2.0
