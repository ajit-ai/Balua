Balua Principles
================

Source: ``spec/BLRS-v1.0.md:10`` (Ch.1 Design Goals), ``README.md:7``.

Motto: **One Language. Every Silicon.**

1. Zero-cost abstractions
-------------------------

High-level constructs (``Tensor``, ``stream<T>``, ``spawn``/``chan``,
closures, traits, const generics) compile to optimal machine code with no
runtime tax. MIR inlining + ``Pure`` effect elision
(``crates/baluac/src/types/effect_system.rs:1``), LLVM ``O3`` + LTO/PGO/BOLT.

2. Hardware-first primitives
----------------------------

Hardware is not a library — it is syntax. ``@hw::gpu`` / ``npu`` / ``fpga``
/ ``quantum`` / ``cpu`` / ``embedded`` annotate regions
(``spec/grammar/balua.ebnf:13``); ``kernel fn`` / ``circuit fn`` /
``tensor fn`` / ``qubit[N]`` / ``v512<T>`` / ``Tensor<T,Shape>`` are
first-class. Each region lowers to its real backend
(``crates/baluac/src/backend/mod.rs:21``) and ``baluald`` stitches a
fat binary (``spec/abi/balua-abi-v1.md:21``).

3. Memory safety without GC
---------------------------

Single owner, move semantics, ``&T`` / ``&mut T``, ``box<T>::new``,
lifetimes ``'a``, cross-device lifetimes, ``Send``/``Sync`` inference.
Use-after-move → ``E_USE_AFTER_MOVE``, borrow conflict →
``E_BORROW_CONFLICT``, type mismatch → ``E_TYPE_MISMATCH``
(``spec/NFR-roadmap-v2.md:9``). No garbage collector; suitable for
``no_std`` / bare-metal (``arm-none-eabi``).

4. Unified toolchain
--------------------

One driver ``balua`` (``crates/baluac/src/main.rs:11``) + one package
manager ``blpkg`` (``crates/blpkg/src/main.rs:1``) + seven DX tools
(``tools/balua-fmt``, ``lsp``, ``dbg``, ``prof``, ``doc``, ``test``,
``bindgen``). JSON diagnostics for IDEs, compile events + profile JSON for
observability, deterministic ``fmt``/``lint``.

5. Determinism & reliability
----------------------------

Deterministic builds (reproducible binaries), fuzzed lexer/parser,
formal contracts ``#[requires]`` / ``[ensures]`` → Frama-C/CBMC
(``crates/baluac/src/safety/formal_verification.rs:1``),
WCET/stack verification (``crates/baluac/src/safety/wcet.rs:1``,
``crates/baluac/src/safety/stack_analysis.rs:1``).

6. Interoperability
-------------------

C11 ABI stable, ``extern "C/C++/rust/cuda/opencl"``, Python embedding,
VHDL/Verilog, WASM, ``balua-bindgen`` → ``.blh``. Mangling
``_Bl<N>...`` with ``#[export_c]`` escape hatch
(``spec/abi/balua-abi-v1.md:3``).

7. Safety-critical certifiability
---------------------------------

``--safety-profile=AUTOSAR_CPP14|DO-178C|IEC61508`` rejects
heap/recursion/unbounded loops; ``#[max_stack]`` / ``#[wcet_cycles]`` /
``#[interrupt_handler]`` / ``#![no_panic]``; MISRA checker
(``crates/baluac/src/safety/misra.rs:1``); SBOM ``Bom.toml`` + provenance +
checksums (supply chain).

8. Simulation-first portability
-------------------------------

Every ``std::hal`` function has a host fallback so
``#[test(sim=true)]`` passes without silicon
(``README.md:67``, ``tests/hal/sim_tests.bl:1``).
Cross-arch (``x86_64/aarch64/riscv64/arm-none-eabi``),
cross-backend (CUDA PTX / SPIR-V / HLS / QASM / MLIR),
cross-OS via portable runtime.

9. Self-hostability & scalability
---------------------------------

``baluac`` in Rust today, self-hostable in Balua within 3 major versions
(``README.md:96``). ``selfhost/lexer.bl`` + ``parser.bl`` already mirror
the compiler; ``ast.bl`` / ``mir.bl`` / ``semantic.bl`` are next.
Parallel / incremental compilation, ``codegen-units=1`` for release LTO.

10. Open governance
-------------------

MIT OR Apache-2.0, Language Design Authority change control,
``CONTRIBUTING.md``, ``GIT_WORKFLOW.md:3``
(``main`` ← ``develop`` ← ``phase/*``), per-phase tags.
