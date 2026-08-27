# 06_quantum_grover — Grover 4-qubit unstructured search, OpenQASM3. Target: lpkg build --target quantum:ibm:fake_manila.

Grover 4-qubit unstructured search, OpenQASM3. Target: lpkg build --target quantum:ibm:fake_manila.

Build:
``powershell
cargo run -p baluac -- examples/06_quantum_grover.bl --emit-llvm
cargo run -p baluac -- examples/06_quantum_grover.bl --emit-mir
blpkg build
``
Zero warnings required (Section 11 constraint).

