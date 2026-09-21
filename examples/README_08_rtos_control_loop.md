# 08_rtos_control_loop — PID 10kHz STM32H7, #![no_std] #![no_panic], max_stack+wcet. Target: blpkg cross arm-none-eabi.

PID 10kHz STM32H7, #![no_std] #![no_panic], max_stack+wcet. Target: blpkg cross arm-none-eabi.

Build:
``powershell
cargo run -p baluac -- examples/08_rtos_control_loop.bl --emit-llvm
cargo run -p baluac -- examples/08_rtos_control_loop.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

