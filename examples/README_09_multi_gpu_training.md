# 09_multi_gpu_training — Distributed SGD 8 GPUs via NCCL.

Distributed SGD 8 GPUs via NCCL.

Build:
``powershell
cargo run -p baluac -- examples/09_multi_gpu_training.bl --emit-llvm
cargo run -p baluac -- examples/09_multi_gpu_training.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

