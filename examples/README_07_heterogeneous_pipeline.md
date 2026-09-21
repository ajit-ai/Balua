# 07_heterogeneous_pipeline — CPU?GPU?NPU?CPU end-to-end, single binary via baluald fat-binary.

CPU?GPU?NPU?CPU end-to-end, single binary via baluald fat-binary.

Build:
``powershell
cargo run -p baluac -- examples/07_heterogeneous_pipeline.bl --emit-llvm
cargo run -p baluac -- examples/07_heterogeneous_pipeline.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

