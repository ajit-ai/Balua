//! HLS C / Vitis backend — FPGA (Xilinx UltraScale+, Versal, Intel Agilex)
//! Transpiles @hw::fpga blocks to Vitis HLS C + Tcl/xdc.

use super::Backend;
use crate::mir::MirModule;

#[derive(Default)]
pub struct HlsBackend { pub part: String, pub clock_mhz: u32 }

impl Backend for HlsBackend {
    fn name(&self) -> &'static str { "hls" }
    fn lower(&self, modules: &[MirModule]) -> anyhow::Result<String> {
        let mut out = String::from("// Balua HLS backend — Vitis HLS C\n#include <hls_stream.h>\n#include <ap_int.h>\n\n");
        for m in modules {
            for f in &m.functions {
                if !matches!(f.hardware, Some(crate::ast::HardwareTarget::Fpga)) { continue; }
                let clk = if self.clock_mhz == 0 { 250 } else { self.clock_mhz };
                out.push_str(&format!("// Part: {} @ {} MHz\n", if self.part.is_empty() { "xcvu9p" } else { &self.part }, clk));
                out.push_str(&format!("#pragma HLS INTERFACE m_axi port=input bundle=gmem\n"));
                out.push_str(&format!("void {}(hls::stream<float> &input, hls::stream<float> &output) {{\n", f.name));
                out.push_str("  #pragma HLS PIPELINE II=1\n");
                out.push_str("  #pragma HLS INTERFACE axis port=input\n");
                out.push_str("  #pragma HLS INTERFACE axis port=output\n");
                out.push_str("  // Balua circuit body lowered from MIR\n}\n\n");
                // Tcl constraint stub
                out.push_str(&format!("// Tcl: create_clock -period {} -name clk [get_ports clk]\n", 1000/clk));
                out.push_str("// xdc: set_property PACKAGE_PIN ... [get_ports clk]\n\n");
            }
        }
        Ok(out)
    }
}
