Development and Contributing
==============================

Branch model (``GIT_WORKFLOW.md``)
----------------------------------

``main`` (protected, production) ← ``develop`` (integration) ←
``phase/*`` / ``feature/*``. Work lands on ``develop`` (or a phase
branch), is pushed, then merged ``develop → main`` with ``--no-ff`` and a
``merge:`` subject, then tagged (``phase-N``, ``v0.N``). Never commit
directly to ``main``. Full contributor rules: ``CONTRIBUTING.md``.

CI (``.github/workflows/ci.yml``)
----------------------------------

- ``build`` (windows): ``cargo build --workspace``; lexer/parser/semantic/
  safety/CLI/fuzz suites; ``--lib`` goldens; ``balua-fmt``/``balua-test``
  suites; end-to-end execution suite (links and runs programs);
  ``--emit-mir`` smoke on ``01_hello_hardware``; ``-o hello-ci.exe`` link
  smoke.
- ``fmt`` (windows): ``balua-fmt --check`` over all in-repo ``.bl`` files
  (``hello.bl``, ``keyword_test.bl``, ``examples``, ``tests``,
  ``tests/hal``, ``selfhost``, ``std``, ``std/hal``, ``std/ffi``).
- ``test-runner`` (windows): ``balua-test`` on ``tests/pass.bl`` plus a
  ``--bench=2`` timing pass.
- ``blpkg`` (windows): build the manager; ``--help`` smoke.
- ``docs`` (ubuntu): install ``docs/requirements.txt`` and run
  ``sphinx-build -b html -W docs docs/_build/html`` — warnings fail the
  build.

Local verification
------------------

::

  cargo build --workspace
  cargo test -p baluac --test lexer_tests --test parser_tests --test semantic_tests --test safety_tests --test cli_tests --test fuzz_smoke
  cargo test -p baluac --lib
  cargo test -p balua-fmt -p balua-test
  cargo test -p baluac --test execution
  cargo test -p blpkg
  cargo run -q -p balua-fmt -- --check hello.bl keyword_test.bl examples tests tests/hal selfhost std std/hal std/ffi
  cargo run -q -p balua-test -- tests/pass.bl
  cargo run -q -p baluac --bin balua -- hello.bl -o hello.exe

Fuzz smoke and soak
-------------------

``crates/baluac/tests/fuzz_smoke.rs``: fixed-seed (``0xBA10A``) token soup
plus every in-repo ``.bl`` file as seed corpus. Contract: lexing and
parsing never panic and token streams stay Eof-terminated; diagnostics are
allowed. Scale via ``BALUA_FUZZ_CASES`` (same seed extends the sequence);
triage via ``BALUA_FUZZ_SKIP``/``BALUA_FUZZ_LOG``/``BALUA_FUZZ_DUMP``. The
M3 soak ran the full sequence to the one-hour volume with zero failures.

Reproducibility
---------------

``.o`` objects are byte-identical across repeated builds (Cranelift
emission is deterministic). Linked executables are byte-identical via
``-Wl,--no-insert-timestamp -Wl,--build-id=none`` on GNU toolchains
(verified across time gaps); MSVC ``cl`` linking is unverified and keeps
prior behavior.

Local documentation
-------------------

Hand mirrors: open ``docs/index.html`` in a browser (``file://`` works;
no server needed). Sphinx build (not committed)::

  pip install -r docs/requirements.txt
  sphinx-build -b html -W docs docs/_build/html

Build output (``docs/_build/``) is git-ignored. The ``Pages`` workflow
(``.github/workflows/pages.yml``) rebuilds with ``-W`` and deploys to
GitHub Pages on every ``main`` push touching ``docs/**`` (enable once
under repo Settings → Pages → Source: GitHub Actions).
