Releases and Changelog
======================

Derived from git history (tags and subjects); most recent first. The
``v1.0.0``/``phase-5…9`` tags predate grounded verification and record
scaffold milestones, not verified artifacts — see :doc:`roadmap`.

Recent history
--------------

- M3 closeout (develop, this change) — ``balua-fmt``/``balua-test``
  workspace tools, CLI + JSON schema freeze tests, fixed-seed fuzz smoke
  and soak, deterministic GNU-linked executables, repo-wide format
  dogfood, CI ``fmt``/``test-runner`` jobs.
- ``2ce74c4`` — M2 safety-min fixes: module splice, attribute plumbing,
  exact heap matching, SCC loop analysis, tier-uniform enforcement,
  semantic double-record fix.
- ``d552a49`` — M1 ``blpkg``-local: real build/run/test via the ``baluac``
  library, semver resolution, ``bom-full``, roadmap-detail docs.
- ``1bc438d`` — docs go local-only (Pages deployment removed).
- ``dcadd59`` — CI with execution suite, ``blpkg`` smoke, docs check.
- ``da01af8`` — Phase-2 close: ``TypeTable`` wiring, ``match`` lowering,
  17 execution cases, language docs.
- ``3340e6c`` — merge ``phase/2-cranelift``: Cranelift CPU backend with
  linked executables.
- ``77a2af1`` — Cranelift backend producing linked executables.
- ``cbe93af`` / ``cc7d74c`` — Phase 1: ``InferenceEngine`` +
  ``BorrowChecker`` wired into semantic analysis (8 tests).
- ``3bf87ef`` and earlier — legacy phase merges (scaffold era).

Tags on record
--------------

``phase-1-semantic``, ``v1.0.0``, ``phase-9``, ``v0.9.0``, ``phase-8``,
``phase-7``, ``v0.8.0``, ``phase-6``, ``v0.7.0``, ``phase-5``, ``v0.6.0``,
``v0.5.0``, ``v0.4.0``, ``phase-2b``, ``v0.3.0``, ``v0.2.0``.

Workspace version remains ``0.1.0`` (``Cargo.toml``); no crates.io
publication, binaries, or installers exist. Release process: per-phase
``develop → main`` merge plus tag (``GIT_WORKFLOW.md``).
