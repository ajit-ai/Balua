# 10_balua_os_kernel — Minimal kernel: page allocator, scheduler, syscall table, no_std.

Minimal kernel: page allocator, scheduler, syscall table, no_std.

Build:
``powershell
cargo run -p baluac -- examples/10_balua_os_kernel.bl --emit-llvm
cargo run -p baluac -- examples/10_balua_os_kernel.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

