Balua Safety Analysis
=====================

This document describes the M2 safety analysis exactly as implemented.
Nothing here is aspirational: every behavior names its implementation file
and diagnostic code.

Where analysis runs
-------------------

Safety operates on lowered MIR (``crates/baluac/src/mir.rs:62``), after
semantic analysis and before linking. The driver (``crates/baluac/src/main.rs``)
runs ``safety::check_modules`` over all MIR modules when
``--safety-profile`` is given; violations join the diagnostics list, print,
and block ``-o`` output (fail-closed). Without ``--safety-profile`` no safety
check runs. Implementation: ``crates/baluac/src/safety/analyze.rs:1``.
Tests: ``crates/baluac/tests/safety_tests.rs:1`` (13 tests).

Safety profiles
---------------

``--safety-profile`` accepts ``AUTOSAR_CPP14``, ``DO-178C``, ``IEC61508``
(case-insensitive, ``-``/``_`` variants accepted), or ``none`` (no
enforcement). Parsing: ``safety::parse_profile``.

Unknown names fail closed with ``E_SAFETY_PROFILE``::

  balua hello.bl --safety-profile=bogus
  # error: unknown safety profile 'bogus' (expected AUTOSAR_CPP14|DO-178C|IEC61508|none)

All three real profiles currently enforce the same three structural rules
(heap, recursion, unbounded loops); they differ only in the label carried in
messages. Certification evidence beyond these checks does not exist yet.

Heap detection (``E_SAFETY_HEAP``)
-----------------------------------

A function is heap-flagged if its MIR contains ``Alloca``,
``HwIntrinsic``, ``MoveToDevice``, or a call to exactly ``move_to_device``,
``box``, ``alloc``, ``malloc``, ``calloc``, or ``realloc``. Matching is
exact (case-sensitive): user functions such as ``allocate`` are clean
(regression-tested). These are dynamic-lifetime operations; the check does
not track actual allocators.

Recursion detection (``E_SAFETY_RECURSION``)
----------------------------------------------

A call graph is built from MIR ``Call`` instructions restricted to defined
functions; any function that can reach itself — directly or indirectly — is
flagged. Example::

  fn f(n: i32) -> i32 { f(n) }   # E_SAFETY_RECURSION under any profile

Loop analysis (``E_SAFETY_LOOP``)
---------------------------------

Reachable control-flow graphs are decomposed into Kosaraju strongly
connected components (linear time). A component with a cycle (more than one
node, or a self-edge) and no successor outside itself is an unbounded loop::

  fn main() -> i32 { loop { } 0 }   # E_SAFETY_LOOP

Branch conditions defined directly by an integer constant are folded:
``while true`` is unbounded, ``while false`` is accepted. Non-constant
conditions always count as exits, so analysis is structural, not semantic —
it proves shape, not termination.

Stack accounting (``E_SAFETY_STACK``)
--------------------------------------

Estimate per function: ``32 + 4 * (params + distinct dest registers)``
bytes. Always reported (``--verbose`` prints the per-function table).
Enforced only against a limit, resolved as: ``#[max_stack(N)]`` attribute
if present, else ``--safety-stack-limit N``. A bare
``--safety-stack-limit`` without ``--safety-profile`` warns and does nothing.

WCET accounting (``E_SAFETY_WCET``)
-----------------------------------

``#[wcet_cycles(N)]`` bounds an instruction-count estimate (one cycle per
MIR instruction). This is an intentionally crude structural proxy, not a
timing model: it is documented as such in the implementation. Exceeding the
bound fails the build. Without the attribute, no WCET check runs.

Safety attributes
-----------------

Leading ``#[...]`` annotations are collected by the parser
(``crates/baluac/src/parser.rs``), stored on ``FnDecl.attrs``
(``crates/baluac/src/ast.rs``), carried into ``MirFunction.attrs``, and
read by the safety pass. Supported: ``#[max_stack(N)]``,
``#[wcet_cycles(N)]``. Anything else yields a ``W_UNKNOWN_ATTR`` warning in
semantic analysis. Attributes on ``kernel``/``circuit`` declarations are
currently dropped (hardware regions are skipped — see below).

Uniform tier policy
-------------------

Enforcement does not depend on ``Safe``/``Unsafe``/``Trusted`` tiers
(``crates/baluac/src/ast.rs``): a profile describes guarantees about the
code, not about function labels, so an ``unsafe`` function with an
unbounded loop is still rejected (tested).

Hardware-targeted functions
---------------------------

Functions annotated ``@hw::gpu/fpga/npu/quantum/embedded`` (anything but
``@hw::cpu`` or plain) are skipped: a CPU profile does not govern
hardware regions, which have their own backends and validators (post-GA).
Skipped names are listed in the verbose report; ``checked_functions``
counts only checked CPU functions.

Clean-run notice
----------------

A clean profile run prints to stderr (stdout stays clean for ``--emit-*``)::

  safety profile AUTOSAR_CPP14: 2 functions checked (0 hardware-targeted skipped), 0 violations

Diagnostics reference
---------------------

``E_SAFETY_HEAP``, ``E_SAFETY_RECURSION``, ``E_SAFETY_LOOP``,
``E_SAFETY_STACK``, ``E_SAFETY_WCET``, ``E_SAFETY_PROFILE`` — all
``Severity::Error`` with source spans from ``MirFunction.span`` (stack
overflow resolution falls back to no span only if attribution fails).
``formal_verification.rs`` (Frama-C/CBMC) remains emit-only and is not part
of enforcement.

What safety is not
------------------

Distinguish four things: (1) compiler safety analysis above — real for the
CPU subset; (2) profile policy — currently the same three rules under three
names; (3) stack/WCET accounting — rough estimates, not measurements;
(4) hardware/runtime behavior — nothing here executes on accelerators or
proves timing. WCET/stack numbers must not be cited as certification
evidence.
