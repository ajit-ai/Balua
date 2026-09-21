Balua Features
==============

Authoritative sources: ``spec/BLRS-v1.0.md:1`` (Ch.1-15 + App.A-D),
``spec/grammar/balua.ebnf:1``, ``spec/abi/balua-abi-v1.md:1``,
``README.md:52``, ``crates/baluac/src/main.rs:11``,
``crates/baluac/src/backend/mod.rs:21``.

1. Language core
----------------

- Statically-typed, compiled system language for heterogeneous compute.
- File types: ``.bl`` source, ``.blh`` headers, ``Balua.toml`` manifest,
  compiler ``balua.exe`` / ``baluac`` (see ``crates/baluac/src/main.rs:11``),
  package manager ``blpkg`` / ``BPM.exe``, REPL ``balua``.
- Lexical structure (BLRS Ch.2): UTF-8, Unicode identifiers,
  ``KEYWORD``, ``IDENTIFIER``, ``LITERAL_INT/FLOAT/STR/BOOL``,
  ``OPERATOR``, ``DELIMITER``, ``COMMENT``,
  ``HARDWARE_DIRECTIVE`` (``@hw::``), ``ANNOTATION`` (``#[...]``, ``#pragma``).
- ~40 keywords (App.A): ``fn let mut const struct enum trait impl mod module
  use if else for while loop match return break continue unsafe async await
  spawn chan select extern type where pub self Self super crate box move in
  as is kernel circuit tensor qubit true false``.
- Operators (App.B, highest to lowest): ``* / %`` → ``+ -`` → ``<< >>`` →
  ``&`` → ``^`` → ``|`` → ``== != < > <= >=`` → ``&&`` → ``||`` →
  ``= +=`` → ``..`` → ``-> => ::``.
- Statements & control flow (Ch.5): ``let``, ``if/else``, ``for/while/loop``,
  ``match``, ``return``, ``spawn``, ``chan``, ``select!``, ``unsafe``.
- Functions & closures (Ch.6): ``fn``, ``async fn``, ``extern "C" fn``,
  ``kernel fn``, ``circuit fn``, ``tensor fn``; closures capture by
  borrow/move.
- Effects (Ch.4): ``fn foo() -> T / Pure | IO | HardwareIO | Unsafe`` —
  callee effects must be subsumed by caller.
- Modules & packages (Ch.7): ``module``, ``use hw::cuda::Grid;``,
  cargo-like ``Balua.toml`` with ``[hardware]`` section
  (``Balua.toml:12``: cpu/gpu/npu/fpga/quantum + ``[profile.debug/release]``).

2. Type system
--------------

- Primitives: ``i8..i128``, ``u8..u128``, ``f16/f32/f64/f128``,
  ``fp16/bf16/tf32``, ``bool``, ``char``, ``qubit[N]``,
  ``*mut T`` / ``*const T`` / ``&T`` / ``&mut T``,
  ``v128<T>`` / ``v256<T>`` / ``v512<T>`` (see ``spec/BLRS-v1.0.md:19``).
- Composites: struct, enum, trait, arrays, slices,
  ``Tensor<T,Shape>``, ``stream<T>``.
- Hardware-parameterised: ``GpuTensor<T,Shape,Backend: GpuBackend>``,
  ``FpgaBuf<T,Depth,Clock>``.
- Dependent / const generics:
  ``fn matmul<const M,K,N>(A: Matrix<f32,M,K>, B: Matrix<f32,K,N>)``
  — dimension mismatch is a compile-time error.
- Grammar reference: ``spec/grammar/balua.ebnf:20`` (Type rules).
- Semantic analysis: HM inference (``crates/baluac/src/types/inference.rs:1``)
  + ownership/borrow (``crates/baluac/src/types/borrow_checker.rs:1``)
  + lifetimes (``crates/baluac/src/types/lifetime.rs:1``)
  + hardware types (``crates/baluac/src/types/hardware_types.rs:1``)
  + effects (``crates/baluac/src/types/effect_system.rs:1``).
  Wired in ``crates/baluac/src/semantic.rs:1`` — returns
  ``(Vec<Diagnostic>, TypeTable)`` with ``E_TYPE_MISMATCH``,
  ``E_BORROW_CONFLICT``, ``E_USE_AFTER_MOVE``.

3. Memory model & ownership
---------------------------

- Single owner, move semantics, ``&T`` / ``&mut T``, ``box<T>::new``,
  lifetimes ``'a`` (BLRS Ch.8).
- Hardware transfer ``x.move_to_device()`` invalidates ``x``.
- Cross-device lifetimes checked.
- Grammar: ``spec/grammar/balua.ebnf:44`` (BorrowExpr / OwnershipExpr).

4. Compiler pipeline
--------------------

- Stages (``README.md:52``):
  ``.bl`` → Lexer (Unicode, ``@hw::`` tokens,
  ``crates/baluac/src/lexer.rs:1``) → Parser (recursive-descent, typed AST,
  ``crates/baluac/src/parser.rs:1``, ``crates/baluac/src/ast.rs:1``) →
  Semantic (HM + ownership + cross-device lifetimes + hw validation,
  ``crates/baluac/src/semantic.rs:1``) → MIR (SSA, hw-annotated BBs,
  ``crates/baluac/src/mir.rs:1``) → Backends → ``baluald`` fat-binary
  (``crates/baluac/src/linker/baluald.rs:1``).
- Diagnostics: ``crates/baluac/src/diagnostics.rs:1`` with
  ``Diagnostic.to_json()``, ``Span``, ``with_code``; CLI
  ``--json-diagnostics`` for IDEs.
- CLI (``crates/baluac/src/main.rs:11``):
  ``--emit-llvm``, ``--emit-mir``, ``--emit-clif``,
  ``--target x86_64|aarch64|riscv64|arm-none-eabi``,
  ``--backend <hw>``, ``--cpu-backend llvm|cranelift``,
  ``-o <exe>``, ``--keep-object``, ``--safety-profile``,
  ``--verbose`` (compile events + profile JSON).

5. Backends (one language, every silicon)
-----------------------------------------

Selection: ``crates/baluac/src/backend/mod.rs:21`` ``select_backend(hw)``.

- LLVM (``crates/baluac/src/backend/llvm.rs:1``):
  ``x86_64/aarch64/riscv64/arm-none-eabi``, LTO/PGO/BOLT,
  string-builder IR + optional ``llvm-sys 18`` via ``--features llvm``,
  ``O3``, auto-vectorize AVX-512, thin LTO, per-arch datalayout.
- Cranelift (``crates/baluac/src/backend/cranelift.rs:1``):
  fast debug backend for 4GB hosts; ``--cpu-backend cranelift``,
  ``--emit-clif``; object emission via
  ``crates/baluac/src/backend/object_emit.rs:1`` →
  ``build_executable(&mir, out, opt, keep_object)`` producing linked ``.exe``.
- PTX (``crates/baluac/src/backend/ptx.rs:1``): SM 70-90, host stubs,
  ``wmma`` instructions, ``cudaLaunchKernel``.
- SPIR-V (``crates/baluac/src/backend/spirv.rs:1``): 1.6, Vulkan compute /
  OpenCL 3.0, ``spirv-val``.
- HLS C (``crates/baluac/src/backend/hls.rs:1``): Vitis
  (UltraScale+/Versal/Agilex) + Tcl/xdc, ``#pragma HLS DATAFLOW``, ``II=1``.
- OpenQASM (``crates/baluac/src/backend/openqasm.rs:1``): 3.0 / Q#,
  IBM/Azure/IonQ/Rigetti, gates ``h/cx/t/s/x/y/z``, ``Rx/Ry/Rz``, Toffoli.
- MLIR (``crates/baluac/src/backend/mlir_dialect.rs:1``): custom ``balua``
  dialect hub.
- Linker ``baluald``: packs ELF + CUBIN + SPIR-V + bitstream + QASM with JSON
  manifest ``__balua_hw_manifest`` (see ``spec/abi/balua-abi-v1.md:21``).
- ABI (App.D): C ABI stable, mangling ``_Bl<N><module><fn><types>``,
  ``#[export_c]`` disables mangling; PTX ``.visible .entry``;
  SPIR-V ``OpEntryPoint Kernel``; HLS ``m_axi``/``axis``; NPU ONNX NCHW;
  QASM ``qubit[N]`` + measure.

6. Hardware abstractions (``@hw::``)
------------------------------------

- Annotations (BLRS Ch.9): ``@hw::gpu(backend=cuda, sm=90)``,
  ``@hw::fpga(target=ultrascale_plus, clock_mhz=250)``,
  ``@hw::npu(backend=onnx)``, ``@hw::quantum(backend=openqasm3, qubits=5)``,
  ``@hw::cpu``, ``@hw::embedded``.
- Grammar: ``spec/grammar/balua.ebnf:13`` (HardwareAnnot / HwParams /
  KernelDecl / CircuitDecl).
- Backend selection per annotated region; mixed-hardware programs compile
  each region separately then stitch.

7. Concurrency & async
----------------------

- Primitives (BLRS Ch.10): ``spawn fn``, ``chan<T>``, ``select!``,
  ``Future`` / ``async`` / ``await``,
  ``Mutex`` / ``RwLock`` / ``Condvar`` per ``std::sync``.
- ``std/sync.bl:1``, ``std/future.bl:1``, ``std/thread.bl:1``.
- ``Send`` / ``Sync`` auto-inferred, ``@hw::gpu`` ``Send`` bound enforced.
- Work-stealing scheduler, async I/O (``std/net.bl:1``),
  ``examples/01b_parallelism.bl:1``.

8. Standard library
-------------------

BLRS Ch.13. All ``no_std`` compatible with ``no_std`` feature.
``std/`` contains:

- ``core.bl``, ``core/``, ``mem.bl``, ``alloc.bl``, ``collections.bl``,
  ``string.bl``, ``io.bl``, ``fs.bl``, ``net.bl``, ``thread.bl``,
  ``sync.bl``, ``atomic.bl``, ``time.bl``, ``math.bl``, ``simd.bl``,
  ``tensor.bl``, ``hal/``, ``ffi/``, ``panic.bl``, ``test.bl``,
  ``fmt.bl``, ``iter.bl``, ``future.bl``, ``log.bl``.
- HAL (``std/hal/``): ``cpu.bl``, ``gpu.bl``, ``npu.bl``, ``fpga.bl``,
  ``quantum.bl``, ``embedded.bl``, ``memory.bl`` — every function has a
  simulation fallback for ``#[test(sim=true)]`` on hosts without hardware.
  See ``tests/hal/sim_tests.bl:1``.

9. FFI & interoperability
-------------------------

- BLRS Ch.11: ``extern "C"``, ``extern "C++"``, ``extern "rust"``,
  ``extern "cuda"``, ``extern "opencl"``, Python embedding,
  VHDL/Verilog, WASM ``wasm32-unknown-unknown``.
- Tool ``balua-bindgen`` emits ``.blh`` (``tools/balua-bindgen/README.md:1``).
- Grammar: ``spec/grammar/balua.ebnf:64`` (ExternABI / FFIDecl).

10. Safety & real-time profiles
-------------------------------

- BLRS Ch.12: ``baluac --safety-profile=AUTOSAR_CPP14|DO-178C|IEC61508``
  disables heap/recursion/unbounded loops.
- Attributes: ``#[max_stack(N)]``, ``#[wcet_cycles(N)]``,
  ``#[interrupt_handler(IRQ=N)]``, ``#[requires]`` / ``[ensures]``,
  ``#![no_panic]``, ``#[sandbox]``, ``constant-time`` for crypto.
- Modules: ``crates/baluac/src/safety/misra.rs:1``,
  ``crates/baluac/src/safety/wcet.rs:1``,
  ``crates/baluac/src/safety/stack_analysis.rs:1``,
  ``crates/baluac/src/safety/formal_verification.rs:1``
  (Frama-C ACSL + CBMC harness).
- SBOM: ``Bom.toml`` via ``crates/blpkg/src/bom.rs:1``.
- Example: ``examples/01c_safety_tiers.bl:1``,
  ``examples/04a_safety_bom.bl:1``,
  ``examples/08_rtos_control_loop.bl:1`` (10kHz PID, certified).

11. Package manager (``blpkg`` / ``BPM``)
-----------------------------------------

- Commands (BLRS Ch.14): ``new/build/run/test/bench/add/publish/doc/fmt/lint/cross``.
- Sources: ``crates/blpkg/src/resolver.rs:1`` (PubGrub),
  ``crates/blpkg/src/builder.rs:1``, ``crates/blpkg/src/registry.rs:1``
  (checksums, lockfile, provenance), ``crates/blpkg/src/bom.rs:1``,
  ``crates/blpkg/src/main.rs:1``.
- Targets: ``blpkg build --target gpu:cuda:sm90``,
  ``fpga:xilinx:ultrascale_plus``, ``quantum:ibm:fake_manila``,
  ``arm-none-eabi`` (bare-metal ``no_std``).
- Registry: ``blpkg.io`` (planned production in Phase 9).

12. Toolchain DX & observability
--------------------------------

- ``tools/balua-fmt/README.md:1`` — AST formatter, ``@hw::`` align.
- ``tools/balua-lsp/README.md:1`` — LSP 3.17 + ``balua-db``,
  autocomplete/goto-def/hover (GPU memory layout).
- ``tools/balua-prof/README.md:1`` — perf/VTune/Tracy + Nsight + Vivado
  unified flame graph.
- ``tools/balua-dbg/README.md:1`` — GDB/LLDB + CUDA-GDB + ChipScope/ILA +
  quantum state viz.
- ``tools/balua-doc/README.md:1`` — ``///`` / ``//!`` → HTML/JSON/MD +
  hw compatibility matrix.
- ``tools/balua-test/README.md:1`` — ``#[test]`` / ``#[test(target=gpu)]`` /
  ``#[bench]``.
- ``tools/balua-bindgen/README.md:1`` — ``.blh`` from C/C++/CUDA/OpenCL.
- Compile events, ``--verbose``, profile JSON
  (``crates/baluac/src/main.rs:64`` ``Profile``).

13. Reference programs
----------------------

- ``examples/``: ``01_hello_hardware.bl`` → ``10_balua_os_kernel.bl``
  (hello → SIMD sort → GPU matmul → NPU classify → FPGA FIR →
  Quantum Grover → heterogeneous pipeline → RTOS loop →
  multi-GPU training → OS kernel) + ``01a_keywords.bl``,
  ``01b_parallelism.bl``, ``01c_safety_tiers.bl``, ``01d_visibility.bl``,
  ``01e_cast_where.bl``, ``02a_llvm_backends.bl``, ``03a_gpu_npu.bl``,
  ``03b_fpga_quantum.bl``, ``04a_safety_bom.bl``,
  ``05a_dx_observability.bl``, ``06a_selfhost.bl``.
- Each ``01-10`` has ``README_*.md``.
- ``selfhost/lexer.bl:1``, ``selfhost/parser.bl:1`` mirror current compiler.
