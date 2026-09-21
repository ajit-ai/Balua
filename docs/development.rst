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
  safety suites; end-to-end execution suite (links and runs programs);
  ``--emit-mir`` smoke on ``01_hello_hardware``; ``-o hello-ci.exe`` link
  smoke.
- ``blpkg`` (windows): build the manager; ``--help`` smoke.
- ``docs`` (ubuntu): install ``docs/requirements.txt`` and run
  ``sphinx-build -b html -W docs docs/_build/html`` — warnings fail the
  build.

Local verification
------------------

::

  cargo build --workspace
  cargo test -p baluac --test lexer_tests --test parser_tests --test semantic_tests --test safety_tests
  cargo test -p baluac --test execution
  cargo test -p blpkg
  cargo run -q -p baluac --bin balua -- hello.bl -o hello.exe

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
