Balua Programming Language Guide
==================================

One Language. Every Silicon. — CPU · GPU · NPU · FPGA · Quantum · Embedded

Sources: ``spec/BLRS-v1.0.md:1``, ``spec/grammar/balua.ebnf:1``,
``spec/abi/balua-abi-v1.md:1``, ``README.md:11``.

This guide documents the Balua language itself: how to write, build, and
run Balua programs. Project planning lives only in ``roadmap.rst``.

1. Getting started
------------------

Files: ``.bl`` source, ``.blh`` headers, ``Balua.toml`` manifest,
compiler ``balua.exe`` / ``baluac``, package manager ``blpkg`` / ``BPM.exe``.

::

  cargo build
  cargo run -p baluac -- hello.bl -o hello.exe
  cargo run -p baluac -- examples/01_hello_hardware.bl --emit-mir
  cargo run -p baluac -- examples/03_gpu_matrix_multiply.bl --emit-llvm --target x86_64 --json-diagnostics

Compiler flags (``crates/baluac/src/main.rs:11``): ``--emit-mir``,
``--emit-llvm``, ``--emit-clif``, ``--target x86_64|aarch64|riscv64|arm-none-eabi``,
``--backend <hw>``, ``--cpu-backend llvm|cranelift``, ``-o <exe>``,
``--keep-object``, ``--json-diagnostics``, ``--safety-profile``,
``--verbose``.

Minimal program (``hello.bl:1``)::

  fn hello() -> i32 {
      let name = "Balua";
      let version = 1;
      version
  }

  fn main() -> i32 {
      let v = hello();
      v
  }

Package manifest (``Balua.toml:1``)::

  [package]
  name = "hello_hardware"
  version = "0.1.0"
  edition = "2026"

  [hardware]
  cpu = { arch = "x86_64", features = ["avx512"] }
  gpu = { backend = "cuda", min_sm = "sm_70" }
  npu = { backend = "onnx", quantization = "int8" }
  fpga = { vendor = "xilinx", part = "xcvu9p" }
  quantum = { backend = "openqasm3", simulator = "aer" }

  [profile.release]
  opt_level = 3
  lto = true

2. Lexical structure
--------------------

UTF-8 source, Unicode identifiers. Tokens: ``KEYWORD``, ``IDENTIFIER``,
``LITERAL_INT/FLOAT/STR/BOOL``, ``OPERATOR``, ``DELIMITER``, ``COMMENT``,
``HARDWARE_DIRECTIVE`` (``@hw::``), ``ANNOTATION`` (``#[...]``, ``#pragma``).

Keywords (``spec/BLRS-v1.0.md:61``): ``fn let mut const struct enum trait
impl mod module use if else for while loop match return break continue
unsafe async await spawn chan select extern type where pub self Self super
crate box move in as is kernel circuit tensor qubit true false``.

3. Modules and packages
-----------------------

::

  module main {
      use std::hal::cpu;
      use std::hal::gpu;

      fn main() -> i32 { 0 }
  }

See ``examples/01_hello_hardware.bl:2``. ``use hw::cuda::Grid;`` imports
hardware types. Each package has ``Balua.toml`` with ``[hardware]`` targets.

4. Types
--------

Primitives: ``i8..i128``, ``u8..u128``, ``f16/f32/f64/f128``,
``fp16/bf16/tf32``, ``bool``, ``char``, ``usize/isize/str``,
``qubit[N]``, ``*mut T`` / ``*const T`` / ``&T`` / ``&mut T``,
``v128<T>`` / ``v256<T>`` / ``v512<T>``.

Composites: ``struct``, ``enum``, ``trait``, arrays ``[T; N]``, slices
``[T]`` / ``&[T]``, ``Tensor<T, Shape>``, ``stream<T>``,
``fn (Args) -> Ret``.

Hardware-parameterised::

  GpuTensor<T, Shape, Backend: GpuBackend>
  FpgaBuf<T, Depth, Clock>

Dependent const generics — dimension mismatch is a compile error::

  fn matmul<const M,K,N>(A: Matrix<f32,M,K>, B: Matrix<f32,K,N>) -> Matrix<f32,M,N>

Grammar: ``spec/grammar/balua.ebnf:20``.

5. Variables, constants, visibility
-----------------------------------

From ``examples/01a_keywords.bl:3``::

  pub const MAX_VAL: i32 = 100;
  priv const INTERNAL: i32 = 42;
  let mut result = 1;
  let name = "Balua";

``pub`` / ``priv`` control visibility. ``const`` is compile-time.
``let mut`` allows reassignment. ``x as f32`` casts::

  pub fn cast_test(x: i32) -> f32 {
      x as f32
  }

Generic bounds use ``where``::

  pub fn where_test<T: Ord, K: Copy>(a: T, b: K) -> T { a }

6. Functions and closures
-------------------------

::

  fn add(a: i32, b: i32) -> i32 { a + b }
  pub fn factorial(n: i32) -> i32 { /* ... */ }
  priv fn helper() -> i32 { 42 }
  async fn fetch() -> i32 { 0 }
  extern "C" fn puts(s: &str) -> i32;
  kernel fn matrix_mul(A: &[f32], B: &[f32], C: &mut [f32], N: u32) -> void { /* ... */ }
  circuit fn grover(q: qubit[5]) -> bit[5] { /* ... */ }
  tensor fn infer(x: Tensor<f32, [224,224,3]>) -> Tensor<f32, [1000]> { /* ... */ }

Closures capture by borrow or ``move``. Effects
(``/ Pure | IO | HardwareIO | Unsafe``) must nest: a ``Pure`` caller
cannot call ``IO``.

7. Control flow
---------------

``if`` / ``else``, ``match``, ``for`` / ``while`` / ``loop``,
``break`` / ``continue``, ``return``. From ``examples/01a_keywords.bl:7``::

  pub fn factorial(n: i32) -> i32 {
      let mut result = 1;
      let mut i = n;
      while i > 1 {
          result = result * i;
          i = i - 1;
      }
      result
  }

  pub fn match_test(x: i32) -> i32 {
      let r = match x {
          1 => 10,
          2 => 20,
          _ => 0
      };
      r + x
  }

  pub fn for_test() -> i32 {
      let mut s = 0;
      for i in 0..10 {
          if i % 2 == 0 { continue }
          s = s + i
      }
      s
  }

  pub fn loop_test() -> i32 {
      let mut i = 0;
      loop {
          i = i + 1;
          if i == 5 { break i }
      }
      i
  }

Operators (high to low): ``* / %`` → ``+ -`` → ``<< >>`` → ``&`` →
``^`` → ``|`` → ``== != < > <= >=`` → ``&&`` → ``||`` → ``= +=`` →
``..`` → ``-> => ::``.

8. Ownership and borrowing
--------------------------

Single owner, move semantics, ``&T`` / ``&mut T``, ``box<T>::new``,
lifetimes ``'a``. Transfer invalidates::

  let buf = box<[f32]>::new(data);
  let gpu_buf = buf.move_to_device();  // `buf` unusable after this
  let r = &gpu_buf;
  let w = &mut gpu_buf;

Diagnostics: type mismatch → ``E_TYPE_MISMATCH``, use-after-move →
``E_USE_AFTER_MOVE``, borrow conflict → ``E_BORROW_CONFLICT``.
Cross-device lifetimes are checked.

9. Concurrency and async
------------------------

From ``examples/01b_parallelism.bl:4``::

  fn spawn_task() -> i32 {
      spawn { let _x = 1 };
      0
  }

  fn chan_test() -> i32 {
      let c = chan i32;
      send c 42;
      let _r = recv c;
      0
  }

  fn select_test() -> i32 {
      select {
          ch1 => { let _a = 1 },
          ch2 => { let _b = 2 },
      };
      0
  }

Plus ``Future`` / ``async`` / ``await``, ``Mutex`` / ``RwLock`` /
``Condvar`` / ``Barrier`` / ``Semaphore`` in ``std/sync.bl``,
``std/future.bl``, ``std/thread.bl``. ``Send`` / ``Sync`` are inferred;
``@hw::gpu`` regions require ``Send``.

10. Safety tiers and attributes
-------------------------------

From ``examples/01c_safety_tiers.bl:4``::

  pub fn safe_add(a: i32, b: i32) -> i32 { a + b }
  pub fn unsafe_div(a: i32, b: i32) -> i32 { a / b }
  pub fn trusted_audit() -> i32 { 42 }

``safe`` is checked, ``unsafe`` needs ``unsafe { }`` at use sites,
``trusted`` needs audit. Real-time attributes::

  #[max_stack(512)]
  #[wcet_cycles(1000)]
  #[interrupt_handler(IRQ=5)]
  #[requires(x > 0)]
  #![no_panic]

CLI: ``balua --safety-profile=AUTOSAR_CPP14|DO-178C|IEC61508``.
SBOM via ``Bom.toml``.

11. Hardware abstractions (``@hw::``)
-------------------------------------

Annotate regions; each lowers to its backend, ``baluald`` stitches a
fat binary (``spec/abi/balua-abi-v1.md:21``)::

  @hw::gpu(backend=cuda, sm=90)
  kernel fn matrix_mul(A: &[f32], B: &[f32], C: &mut [f32], N: u32) -> void {
      let row = thread_id::x();
      let col = thread_id::y();
      C[row * N + col] = A[row] * B[col];
  }

  @hw::fpga(target=ultrascale_plus, clock_mhz=250)
  fn fir_filter(x: stream<f32>) -> stream<f32> { x }

  @hw::npu(backend=onnx)
  tensor fn classify(x: Tensor<f32, [224,224,3]>) -> Tensor<f32, [1000]> { x }

  @hw::quantum(backend=openqasm3, qubits=5)
  circuit fn grover(q: qubit[5]) -> bit[5] { /* h/cx/t/s/x/y/z */ }

  @hw::cpu
  fn sort(a: &mut [f32]) -> void { /* AVX-512 v512<T> */ }

  @hw::embedded
  fn isr() -> void { /* #![no_std] */ }

See ``examples/03_gpu_matrix_multiply.bl:5``.

12. FFI and ABI
---------------

::

  extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> void;
  extern "cuda" fn launch_kernel(grid: Grid, block: Block) -> void;
  extern "rust" fn rust_helper(x: i32) -> i32;

C ABI stable, mangling ``_Bl<N><module><fn><types>``,
``#[export_c]`` disables mangling. WASM ``wasm32-unknown-unknown``,
Python embedding, VHDL/Verilog bridges. ``balua-bindgen`` emits ``.blh``.

13. Standard library
--------------------

All ``no_std`` compatible. ``std/``: ``core``, ``mem``, ``alloc``,
``collections``, ``string``, ``io``, ``fs``, ``net``, ``thread``,
``sync``, ``atomic``, ``time``, ``math``, ``simd``, ``tensor``,
``hal``, ``ffi``, ``panic``, ``test``, ``fmt``, ``iter``, ``future``,
``log``.

HAL (``std/hal/``): ``cpu``, ``gpu``, ``npu``, ``fpga``, ``quantum``,
``embedded``, ``memory`` — every function has a host simulation fallback
so ``#[test(sim=true)]`` passes without silicon.

14. Toolchain usage
-------------------

::

  blpkg new my_app
  blpkg build
  blpkg build --target gpu:cuda:sm90
  blpkg build --target fpga:xilinx:ultrascale_plus
  blpkg build --target quantum:ibm:fake_manila
  blpkg run
  blpkg test

Tools in ``tools/``: ``balua-fmt`` (formatter), ``balua-lsp`` (IDE),
``balua-dbg`` (debugger), ``balua-prof`` (profiler), ``balua-doc``
(``///`` → HTML/JSON/MD), ``balua-test`` (``#[test]`` / ``#[bench]``),
``balua-bindgen`` (``.blh``).

15. Full hello-hardware walkthrough
-----------------------------------

``examples/01_hello_hardware.bl:1``::

  module main {
      use std::hal::cpu;
      use std::hal::gpu;
      use std::hal::npu;
      use std::hal::fpga;
      use std::hal::quantum;

      fn main() -> i32 {
          println("Hello from Balua!");
          println("CPU cores: {}", cpu::core_count());
          println("GPU available: {}", true);
          println("NPU backend: {:?}", npu::detect_backend());
          println("FPGA bitstream loaded: {}", false);
          println("Quantum qubits: {}", 5);
          0
      }
  }

Build and inspect::

  cargo run -p baluac -- examples/01_hello_hardware.bl --emit-mir
  cargo run -p baluac -- examples/01_hello_hardware.bl -o hello.exe --verbose

More programs in ``examples/``: ``01a_keywords`` … ``10_balua_os_kernel``.
Grammar: ``spec/grammar/balua.ebnf:1``. ABI: ``spec/abi/balua-abi-v1.md:1``.
