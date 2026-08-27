# 05_fpga_fir_filter — 256-tap FIR for SDR, Xilinx ZCU104 UltraScale+ @250MHz. Target: lpkg build --target fpga:xilinx:ultrascale_plus (HLS+Vitis).

256-tap FIR for SDR, Xilinx ZCU104 UltraScale+ @250MHz. Target: lpkg build --target fpga:xilinx:ultrascale_plus (HLS+Vitis).

Build:
``powershell
cargo run -p baluac -- examples/05_fpga_fir_filter.bl --emit-llvm
cargo run -p baluac -- examples/05_fpga_fir_filter.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

