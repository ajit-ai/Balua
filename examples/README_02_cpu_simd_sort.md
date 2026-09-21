# 02_cpu_simd_sort — AVX-512 radix sort on 100M ints, vs std::sort. Target: x86_64+aarch64, aluac --target x86_64.

AVX-512 radix sort on 100M ints, vs std::sort. Target: x86_64+aarch64, aluac --target x86_64.

Build:
``powershell
cargo run -p baluac -- examples/02_cpu_simd_sort.bl --emit-llvm
cargo run -p baluac -- examples/02_cpu_simd_sort.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

