# 04_npu_image_classify — MobileNetV3 ONNX top-5. Target: lpkg build --target npu:onnx.

MobileNetV3 ONNX top-5. Target: lpkg build --target npu:onnx.

Build:
``powershell
cargo run -p baluac -- examples/04_npu_image_classify.bl --emit-llvm
cargo run -p baluac -- examples/04_npu_image_classify.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

