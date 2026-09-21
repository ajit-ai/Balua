# 03_gpu_matrix_multiply — CUDA tensor-core GEMM vs cuBLAS. Target: blpkg build --target gpu:cuda:sm90 (PTX).

CUDA tensor-core GEMM vs cuBLAS. Target: blpkg build --target gpu:cuda:sm90 (PTX).

Build:
``powershell
cargo run -p baluac -- examples/03_gpu_matrix_multiply.bl --emit-llvm
cargo run -p baluac -- examples/03_gpu_matrix_multiply.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

