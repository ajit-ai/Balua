# Balua ABI v1 — Calling Convention (Appendix D)

## 1. CPU (LLVM targets)
- Default: C ABI stable (`extern "C"`). Balua `extern "C" fn` is ABI-compatible with C11.
- Balua mangling: `_Bl<N><module><fn><types>` (Itanium-like). `#[export_c]` disables mangling.
- Integers < 64-bit are sign/zero-extended; floats passed in FPRs per target (AAPCS64, SystemV).

## 2. GPU
- PTX: kernels use `.visible .entry` with `.param .u64` pointers; launch via `cudaLaunchKernel`.
- SPIR-V: `OpFunction` kernel entry `OpEntryPoint Kernel`; uniform buffers per Vulkan 1.3.

## 3. FPGA
- HLS: `m_axi` for DRAM, `axis` for streams; auto-generated `xdc` pins; `II=1` default pipeline.

## 4. NPU
- ONNX tensor layout NCHW, int8/fp16 quantization params in side-band metadata.

## 5. Quantum
- OpenQASM 3.0: `qubit[N]` + `bit[N] c = measure q;` ; Q# emission via `operation`.

## 6. Fat Binary
- `baluald` packs: ELF + CUBIN + SPIR-V + bitstream + QASM sections with JSON manifest `__balua_hw_manifest`.
