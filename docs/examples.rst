Balua Examples
==============

21 ``examples/*.bl`` programs plus per-program READMEs for 01–10 and two
repo-level suites. Status below was measured in-tree with
``balua <file> --emit-mir`` (exit 0 = parses and analyzes clean; exit 1 =
diagnostics reported). Only ``hello.bl`` (repo root) has additionally been
linked and executed (exit 1, by design: it returns 1).

Clean (exit 0)
--------------

- ``01_hello_hardware.bl`` — all-silicon hello with HAL imports.
- ``05_fpga_fir_filter.bl`` — FIR filter sketch.
- ``06_quantum_grover.bl`` — Grover search sketch.

Reporting diagnostics (exit 1)
------------------------------

``01a_keywords``, ``01b_parallelism``, ``01c_safety_tiers``,
``01d_visibility``, ``01e_cast_where``, ``02_cpu_simd_sort``,
``02a_llvm_backends``, ``03_gpu_matrix_multiply``, ``03a_gpu_npu``,
``03b_fpga_quantum``, ``04_npu_image_classify``, ``04a_safety_bom``,
``05a_dx_observability``, ``06a_selfhost``,
``07_heterogeneous_pipeline``, ``08_rtos_control_loop``,
``09_multi_gpu_training``, ``10_balua_os_kernel``.

These files document intended programs (keywords, parallelism, safety
tiers, accelerators, RTOS, OS kernel) but use syntax or std APIs beyond
the compilable subset — e.g. ``03_gpu_matrix_multiply`` needs slice types
and ``+=``, which are real parser gaps deliberately left out of scope.
Their READMEs (``examples/README_*.md``) describe intent, not verified
behavior.

Suites and self-host sources
----------------------------

- ``tests/hal/sim_tests.bl`` and ``tests/comprehensive_test.bl`` are
  commentary-style Balua suites not executed by cargo or CI.
- ``selfhost/lexer.bl`` / ``selfhost/parser.bl`` mirror the Rust lexer and
  parser in Balua-like pseudocode (naive tokenizer, thin wrappers); they
  are not compiled by ``baluac`` in tests (see ``selfhost/README.md``).
- Executable guarantees live only in ``crates/baluac/tests/`` (14 Rust
  tests, including 17 end-to-end execution cases).
