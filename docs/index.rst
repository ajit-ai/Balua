Balua Programming Language Documentation
========================================

One Language. Every Silicon. — CPU · GPU · NPU · FPGA · Quantum · Embedded

Version 1.0 | Language Design Authority | 2026

This is the documentation for the Balua programming language itself —
syntax, types, control flow, functions, ownership, hardware abstractions,
standard library, toolchain usage, and examples.
HTML mirrors are provided alongside each ``.rst`` file for direct browsing
without a Sphinx build.

Contents
--------

.. toctree::
   :maxdepth: 2

   language
   features
   principles
   roadmap

Project snapshot (2026-09-21)
-----------------------------

- Repository: ``https://github.com/ajit-ai/Balua``
- Branch: ``develop`` — clean working tree, no stash
- Last commit: ``3340e6c`` — merge ``phase/2-cranelift -> develop``
- Workspace: ``Cargo.toml:1`` — members ``crates/baluac``, ``crates/blpkg``
- Spec: ``spec/BLRS-v1.0.md:1``, Grammar ``spec/grammar/balua.ebnf:1``,
  ABI ``spec/abi/balua-abi-v1.md:1``
- Grounded status ledger: ``spec/NFR-roadmap-v2.md:7``
- Legacy full roadmap: ``spec/NFR-roadmap.md:18``

Quick start
-----------

::

  cargo build
  cargo run -p baluac -- examples/01_hello_hardware.bl --emit-mir
  cargo run -p baluac -- examples/03_gpu_matrix_multiply.bl --emit-llvm --target x86_64 --json-diagnostics
  cargo run -p blpkg -- new my_app
  cargo run -p blpkg -- build --target gpu:cuda:sm90

See ``README.md:11`` for full quick start.

Files in this folder
--------------------

- ``index.rst`` / ``index.html`` — this overview
- ``language.rst`` / ``language.html`` — Balua language guide (syntax to HAL with examples)
- ``features.rst`` / ``features.html`` — complete feature catalogue
- ``principles.rst`` / ``principles.html`` — design principles
- ``roadmap.rst`` / ``roadmap.html`` — roadmap only (phases, no GA planning)

License: MIT OR Apache-2.0. See ``LICENSE``.
