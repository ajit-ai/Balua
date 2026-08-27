# 01_hello_hardware — Detect all silicon, simulation fallback, lpkg run works on any host. Target: CPU only.

Detect all silicon, simulation fallback, lpkg run works on any host. Target: CPU only.

Build:
``powershell
cargo run -p baluac -- examples/01_hello_hardware.bl --emit-llvm
cargo run -p baluac -- examples/01_hello_hardware.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

