Runtime and Packages
====================

There is no Balua runtime. Generated executables are standalone native
binaries; a program communicates by returning ``i32`` from ``main``, which
becomes the process exit code. Strings, collections, concurrency
primitives, and HAL calls have no runtime representation in generated code
(strings lower to null placeholders; ``spawn``/``chan`` lower to nothing
runnable).

Package manager (``blpkg`` / ``BPM``)
-------------------------------------

Implemented crate: ``crates/blpkg/`` (bins ``BPM`` and ``blpkg``,
``src/main.rs:40``).

- ``new <name>`` (real): scaffolds ``Balua.toml`` + ``src/main.bl``.
- ``build [--target T] [--release]`` (real, local-only): parses
  ``Balua.toml``, resolves ``src/main.bl`` (fallback: first ``src/*.bl``),
  compiles via the ``baluac`` library (lex → parse → semantic → MIR →
  linked exe), writes ``target/debug/<name>.exe`` (or
  ``target/release/``). Non-CPU targets print a post-GA note and still
  build the CPU entry. Implementation: ``src/builder.rs``.
- ``run`` (real): builds, then executes the artifact and reports its exit
  code.
- ``test`` (real, minimal): builds, then executes the entry and reports
  ``test(sim)`` with its exit code. There is no Balua-level test harness
  yet; Rust-side suites live in ``crates/baluac/tests/``.
- ``add name[@req]`` (real, local-only): validates the requirement with
  ``semver::VersionReq`` (``src/resolver.rs``); no network, no registry.
- ``publish`` (explicit error): ``blpkg.io`` is post-GA; the command fails
  with that message instead of fake success.
- ``bench``, ``doc``, ``fmt``, ``lint``, ``cross`` (prints): post-GA.
- ``bom`` (partial): ``src/bom.rs`` emits CycloneDX-style TOML;
  ``generate_bom_full`` includes license/source/hash when set.

Manifest convention (``Balua.toml:1``): ``[package]`` name/version/edition,
``[hardware]`` targets, ``[dependencies]`` (local validation only),
``[profile.debug/release]``. The compiler itself does not read the
manifest; ``blpkg`` maps it onto compiler invocations.
