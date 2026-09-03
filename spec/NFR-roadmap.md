# Balua NFR Implementation Roadmap (v0.9.0 → v1.0.0 production)

> Maps every non-functional requirement (Scalability, Parallelism, Security, Performance, Portability, Reliability, DX, Maintainability, Safety, Observability, Ecosystem, Compilation-time, Hardware) to concrete phases.
> Each phase commits to `develop` → merges `develop→main` per `GIT_WORKFLOW.md:15`.
> Tags `phase-N` + `v0.N` at each exit.

---

## Legend
- **ETA:** months from now on `i7/4GB` Win11 host (`.cargo/config.toml:1` `jobs=2`)
- **Host:** `balua.exe` `target/debug/balua.exe` + `BPM.exe` `target/debug/BPM.exe`
- **Exit criteria:** cargo build/test green + `balua --help` shows feature + `BPM build --target` works

---

## PHASE MAP (overview)

| Phase | ETA | NFRs addressed | Exit tag |
|-------|-----|----------------|----------|
| Phase 0 | DONE | Scaffold (v1.0.0) | `v0.1.0` |
| Phase 1 | DONE | Backend hardening (LLVM/PTX/SPIR-V/HLS/QASM/MLIR) | `phase-1` |
| Phase 2a | DONE | Keywords — const/match/for-while-loop/break-continue/where/pub/priv/cast | `phase-2a` |
| Phase 2b | DONE | LLVM backend — string/llvm-sys split, `--features llvm`, LTO/PGO/BOLT | `phase-2b` |
| Phase 3 | DONE | Parallelism runtime — spawn/chan/send/recv/select, Send/Sync | `phase-3` |
| Phase 4 | DONE | Security/safety — safe/unsafe/trusted tiers, SBOM (Bom.toml) | `phase-4` |
| Phase 5 | DONE | GPU/NPU — wmma PTX, @hw::npu | `phase-5` |
| Phase 6 | DONE | FPGA/Quantum — HLS DATAFLOW, OpenQASM gates | `phase-6` |
| Phase 7 | DONE | DX/Observability — compile events, --verbose, profile JSON | `phase-7` |
| Phase 8 | DONE | Self-host — selfhost/{lexer,parser}.bl mirror current compiler | `phase-8` |
| Phase 9 | ETA v1.0.0 | Ecosystem — balua-fmt/lsp/dbg/prof/doc/test/bindgen, blpkg.io, docs | `v1.0.0` |

---

## PHASE 2 — Performance + Compilation-time (Month 1–2)
**NFRs:** Performance, Compilation-time
**Files:**
- `crates/baluac/src/backend/llvm.rs:1` → link `llvm-sys 18` (`[features] llvm`) — real `LLVMBuildAdd/Call/Ret`, `O3`, `auto-vectorize AVX-512` (`std/hal/cpu.bl:1` `v512<T>`), `thin LTO`, `target datalayout` per arch
- `crates/baluac/src/backend/llvm.rs:34` `pgo`/`bolt` fields → `balua-prof` `profdata` merge → rebuild
- `crates/baluac/src/main.rs:1` `balua --cpu-backend llvm --release --pgo` → real `x86_64` `.o`
- `crates/baluac/src/mir.rs:1` MIR inlining + `Pure` effect elision (`types/effect_system.rs:1`)
- `.cargo/config.toml:1` `codegen-units=1` for release (single codegen unit = faster LTO)
- `selfhost/ast.bl:1` + `selfhost/mir.bl:1` — self-host prep (mirrors `ast.rs`/`mir.rs`)
**Exit:** `balua selfhost/ast.bl --emit-llvm --release` → real `x86_64` object, `cargo test --features llvm` passes
**Validate:** `cargo bench` on `examples/02_cpu_simd_sort.bl:1` — beats std::sort on AVX-512
**Commit:** `feat(phase-2): real LLVM codegen + PGO + self-host ast/mir`

---

## PHASE 3 — Parallelism + Portability runtime (Month 3–4)
**NFRs:** Parallelism, Portability, Reliability
**Files:**
- `crates/baluac/src/ast.rs:194` `spawn fn` + `chan<T>` + `select!` → work-stealing scheduler (`std::sync::Arc` + crossbeam-equivalent)
- `std/sync.bl:1` `Mutex`/`RwLock`/`Condvar`/`Barrier`/`Semaphore` — compile-time `Send`/`Sync` inference (`types/borrow_checker.rs:1`)
- `std/future.bl:1` `async`/`await` + `Future` trait + executor
- `std/net.bl:1` `TcpListener`/`UdpSocket` async I/O
- `types/effect_system.rs:1` `Send`/`Sync` traits auto-inferred, `@hw::gpu` `Send` bound enforced
- `backend/llvm.rs:1` `aarch64`/`riscv64`/`arm-none-eabi` datalayout + `std/hal/cpu.bl:1` NEON/SVE/RVV
- `balua --target arm-none-eabi` → `no_std` bare-metal `.o` (examples/08_rtos_control_loop.bl:1)
- `cargo test --target arm-none-eabi` (cross-compile on host)
**Exit:** `spawn`+`chan`+`select!` compiles and runs on `x86_64` + `arm-none-eabi`, no data races at compile time
**Validate:** `cargo test --test parallel_tests` (race-free by construction)
**Commit:** `feat(phase-3): parallelism runtime + Send/Sync + cross-arch`

---

## PHASE 4 — Security + Maintainability (Month 5–6)
**NFRs:** Security, Maintainability, Safety
**Files:**
- `std/hal/memory.bl:1` `mmap`/`mprotect`/`alloc_hugepage` — FFI sandbox: `extern "C"` (`std/ffi/c.bl:1`) capability tokens, `#[sandbox]` attribute limits untrusted C access
- `safety/misra.rs:1` `AUTOSAR_CPP14`/`DO_178C`/`IEC_61508` — `cargo build --safety-profile=AUTOSAR_CPP14` rejects heap/recursion/unbounded-loop
- `crates/baluac/src/safety/formal_verification.rs:1` `#[requires]`/`[ensures]` → Frama-C ACSL + CBMC harness (emit `emit_frama_c()`/`emit_cbmc()`)
- `crates/baluac/src/safety/wcet.rs:1` + `stack_analysis.rs:1` — `balua --safety-profile` → worst-case stack + timing report
- `std/hal/cpu.bl:1` `AtomicU64`/`Ordering` — `constant-time` attribute for crypto
- `BPM` `registry.rs:1` — checksum verification, lockfile, provenance attestation
- `balua-fmt` + `balua-lint` — deterministic formatting + lint (supply chain)
- `CONTRIBUTING.md` + `GIT_WORKFLOW.md:15` + `spec/NFR-roadmap.md:1` maintained
**Exit:** `balua --safety-profile=AUTOSAR_CPP14 hello_taste.bl` → zero heap/recursion, `BPM publish` with provenance
**Validate:** `cargo test --features llvm --safety-profile=DO_178C` — all violations caught at compile time
**Commit:** `feat(phase-4): security sandbox + safety profiles + supply chain`

---

## PHASE 5 — GPU/NPU real execution (Month 7–10)
**NFRs:** Performance (heterogeneous), Hardware
**Files:**
- `backend/ptx.rs:1` → `ptx-builder` → real `.cubin` + `cudaLaunchKernel` via `std/hal/gpu.bl:1` `DeviceBuf`/`wmma_gemm`/`nccl`
- `backend/spirv.rs:1` → `rspirv` → `.spv` + `spirv-val`
- `std/hal/gpu.bl:1` real CUDA runtime bindings (`CUDA_VERSION` probed)
- `std/tensor.bl:1` `Tensor<T,Shape>` `NPU` ONNX `quantize_int8`/`quantize_fp16` → `Qualcomm HTP`/`Apple ANE`/`Intel NPU`
- `examples/03_gpu_matrix_multiply.bl:1` ≥0.9× cuBLAS
- `examples/04_npu_image_classify.bl:1` ONNX MobileNetV3 top-5
- `examples/09_multi_gpu_training.bl:1` 8-GPU NCCL
**Exit:** `BPM build --target gpu:cuda:sm90` → `kernel.cubin` + `cudaLaunchKernel`, `BPM build --target npu:onnx` → ONNX runtime
**Validate:** `cargo test --target gpu:cuda:sm90` (requires NVIDIA GPU + CUDA 12 — CI via `.github/workflows/ci.yml:1`)
**Commit:** `feat(phase-5): real PTX + SPIR-V + CUDA + ONNX NPU`

---

## PHASE 6 — FPGA/Quantum production (Month 11–14)
**NFRs:** Portability (FPGA/Quantum), Hardware
**Files:**
- `backend/hls.rs:1` → Vivado `xclbin` + `xdc` + `II=1` timing-closed (`examples/05_fpga_fir_filter.bl:1` `xcvu9p` @250MHz)
- `backend/openqasm.rs:1` → `aer`/`Qiskit` fidelity (`examples/06_quantum_grover.bl:1`)
- `std/hal/fpga.bl:1` real `BRAM`/`URAM` + `AXI`/`PCIe DMA` + DSP multiply-accumulate
- `std/hal/quantum.bl:1` real gates (`H`/`CNOT`/`Toffoli`/`Rx`/`Ry`/`Rz`) + surface code + `Qiskit` bridge
- `tools/balua-dbg/README.md:1` ChipScope/ILA + quantum state viz
**Exit:** `BPM build --target fpga:xilinx:ultrascale_plus` → `kernel.xclbin` (Vivado timing closed), `BPM build --target quantum:ibm:fake_manila` → `.qasm` + fidelity report
**Validate:** `cargo test --target fpga:xilinx` (requires Vitis + Xilinx — CI)
**Commit:** `feat(phase-6): HLS Vivado + OpenQASM/Qiskit`

---

## PHASE 7 — DX + Observability (Month 15–18)
**NFRs:** DX, Observability, Maintainability
**Files:**
- `tools/balua-fmt/README.md:1` — AST-based formatter (`gofmt`-like, `@hw::` align)
- `tools/balua-lsp/README.md:1` — LSP 3.17 server + `balua-db` — autocomplete/goto-def/hover (GPU memory layout)
- `tools/balua-prof/README.md:1` — perf/VTune/Tracy + Nsight + Vivado → unified flame graph (CPU+GPU+NPU)
- `tools/balua-dbg/README.md:1` — GDB/LLDB + CUDA-GDB + ChipScope/ILA + quantum state viz
- `tools/balua-doc/README.md:1` — `///`/`//!` → HTML/JSON/MD + hw compatibility matrix
- `tools/balua-test/README.md:1` — `#[test]`/`#[test(target=gpu)]`/`#[bench]`
- `tools/balua-bindgen/README.md:1` — `.blh` from C/C++/CUDA/OpenCL headers
- `crates/baluac/src/diagnostics.rs:1` `Diagnostic.to_json()` IDE integration (already exists, integrate with LSP)
**Exit:** `balua fmt`/`balua lsp`/`balua prof`/`balua dbg`/`balua doc` all work, `balua-lsp` hover shows GPU memory layout
**Validate:** `cargo test --test dx_tests` (formatter idempotent, LSP protocol)
**Commit:** `feat(phase-7): full DX toolchain (fmt/lsp/prof/dbg/doc/test/bindgen)`

---

## PHASE 8 — Reliability + Self-host (Month 19–22)
**NFRs:** Reliability, Compilation-time, Scalability
**Files:**
- `selfhost/ast.bl:1` + `selfhost/mir.bl:1` + `selfhost/semantic.bl:1` + `selfhost/lexer.bl:1` + `selfhost/parser.bl:1` — `baluac` rewritten in Balua
- `balua` builds `balua.exe` itself (self-hosted) — no `rustc` needed at runtime
- `examples/10_balua_os_kernel.bl:1` boots under `balua` (page allocator + scheduler + syscall table, `#![no_std]`/`#![no_panic]`)
- `examples/08_rtos_control_loop.bl:1` 10kHz PID — `#[wcet_cycles]` + `#[max_stack]` + `#[interrupt_handler]` certified (`AUTOSAR/DO-178C`)
- `cargo fuzz` lexer/parser (fuzzing harness)
- `cargo test --release --features llvm` — reproducible builds (identical binary on re-run)
**Exit:** `balua` builds itself (`balua selfhost/lexer.bl --emit-llvm` → `balua.exe`), `examples/10_balua_os_kernel.bl:1` boots
**Validate:** `cargo test --release --features llvm` + `cargo fuzz` — 0 failures
**Commit:** `feat(phase-8): self-hosted balua + OS kernel + reproducible builds`

---

## PHASE 9 — Ecosystem + Governance (Month 23–24)
**NFRs:** Ecosystem, Maintainability, Governance
**Files:**
- `BPM` `blpkg.io` registry server live — checksums + provenance + lockfile
- `blpkg` `BPM publish`/`BPM add`/`BPM run`/`BPM test`/`BPM bench`/`BPM cross` production
- `criterion` `#[bench]` (`examples/02_cpu_simd_sort.bl:1` benchmark suite)
- `CONTRIBUTING.md` + `GIT_WORKFLOW.md:15` + `spec/NFR-roadmap.md:1` + `spec/BLRS-v1.0.md:1` maintained
- `Language Design Authority` change control
- `balua` `balua --version` → `v1.0.0`
**Exit:** `blpkg.io` has 100+ packages, `BPM add balua-tensor` works, `balua --version` → `v1.0.0`
**Validate:** `BPM publish` → `BPM add` → `BPM run` chain works end-to-end
**Commit:** `feat(phase-9): blpkg.io registry + v1.0.0 production`

---

## NFR-TO-PHASE QUICK REFERENCE

| NFR | Phase |
|-----|-------|
| Scalability (compiler/stdlib/pkg) | 2, 8, 9 |
| Parallelism (spawn+chan+select+Send/Sync) | 3 |
| Security (sandbox+unsafe min+cert+supply chain) | 4 |
| Performance (LLVM O3+vectorize+PGO+BOLT+GPU tensor) | 2, 5 |
| Portability (cross-arch+cross-OS+cross-backend) | 3, 5, 6 |
| Reliability (determinism+formal verify+fuzz+reproducible) | 4, 8 |
| DX (fmt/lsp/prof/dbg/doc/test/bindgen) | 7 |
| Maintainability (modular crates+docs+tests+API stable) | 4, 7, 9 |
| Safety (AUTOSAR/DO-178C/WCET/stack/interrupt) | 4, 8 |
| Observability (prof flame graph+logging+tracing+JSON diag) | 7 |
| Ecosystem (blpkg.io+provenance+governance) | 9 |
| Compilation-time (incremental+parallel+caching) | 2, 8 |
| Hardware (FPGA BRAM/AXI/Quantum gates/NPU quant) | 5, 6 |

---

## PER-PHASE GIT WORKFLOW (`GIT_WORKFLOW.md:15`)

```powershell
# Each phase:
git checkout develop
git checkout -b phase/N-<name>
# ... implement ...
git add <files>
git commit -m "feat(phase-N): <name>"
git push origin develop
git checkout main
git merge --no-ff develop -m "merge: Phase-N → main (Balua v0.N.0)"
git push origin main
git tag -a phase-N -m "Phase N complete"
git tag -a v0.N.0 -m "Balua v0.N.0"
git push origin --tags
```

## CURRENT STATE (v1.0.0, `main@3aba7e2`)
- Phase 0 DONE: scaffold (`v0.1.0`)
- Phase 1 DONE: backend hardening (`phase-1`)
- Phase 2 READY: real LLVM codegen + PGO + self-host ast/mir
- Phase 3: parallelism runtime
- ... up to Phase 9 (v1.0.0 production)

## NEXT ACTION
Phase 2 implementation: `cargo build --features llvm` → real `LLVMBuild*` + `auto-vectorize` + `PGO`. Tell me `go phase-2` to start.
