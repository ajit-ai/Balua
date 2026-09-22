# balua-test

Compile-and-run test harness. Convention: each target `.bl` file is a
program whose `main` exit code is the verdict — passes iff it compiles,
links, and exits 0 (the same convention self-checking examples use).
`--bench[=N]` reruns targets and reports mean time. There is no
Balua-level unit-test attribute yet (`#[bench]` timing only, post-GA scope
for richer harnesses).

Usage: `cargo run -p balua-test -- [--bench[=N]] <files-or-dirs...>`.

