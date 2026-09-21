Heterogeneous Computing
=========================

This page assembles only established material: spec targets, backend
status as implemented, and example inventory with measured status. No
accelerator execution exists; nothing here describes M3 or later work.

Targets (spec)
--------------

``spec/BLRS-v1.0.md`` Appendix C: CPU ``x86_64/aarch64/riscv64/
arm-none-eabi``; GPU SM70–SM90 CUDA PTX plus SPIR-V 1.6; NPU Qualcomm HTP,
Apple ANE, Intel NPU, Samsung, MediaTek; FPGA UltraScale+, Versal, Agilex;
QPU IBM, Azure, IonQ, Rigetti. Manifest section ``[hardware]`` in
``Balua.toml:12``. ABI notes in ``spec/abi/balua-abi-v1.md:1`` (C-stable
CPU calling, PTX ``.entry``, SPIR-V entry points, HLS ``m_axi``/``axis``,
ONNX NCHW layout, QASM measure, fat-binary manifest
``__balua_hw_manifest``) — documented conventions, of which only the
C-ABI-compatible CPU path executes.

Backend status (implemented)
----------------------------

``crates/baluac/src/backend/``: Cranelift emits runnable host objects;
LLVM/PTX/SPIR-V/HLS/OpenQASM/MLIR emit header/template text (see
:doc:`compiler`). The ``baluald`` fat-binary linker
(``src/linker/baluald.rs:22``) is a print-only stub. Mixed-hardware
programs therefore describe regions (``@hw::``) but only CPU regions run.

Simulation fallback
-------------------

Every ``std/hal/*.bl`` function carries a host fallback returning dummy
values (e.g. ``core_count() -> 1``), so ``#[test(sim=true)]``-style checks
pass without silicon. The compiler never reads ``std/``; the HAL is
currently documentation-grade Balua text, not linked code. See
``tests/hal/sim_tests.bl`` (not executed by cargo).

Example status (measured)
-------------------------

``cargo run -q -p baluac --bin balua -- <file> --emit-mir`` exit codes,
measured in-tree: ``01_hello_hardware`` clean; ``05_fpga_fir_filter`` and
``06_quantum_grover`` clean; the other 18 of 21 ``examples/*.bl`` files
report diagnostics (including ``03_gpu_matrix_multiply``, whose slice
types and ``+=`` are real parser gaps, out of scope). Only ``hello.bl``
was linked and executed (exit 1, by design). Per-example READMEs
(``examples/README_*.md``) describe intended behavior, not verified
behavior.
