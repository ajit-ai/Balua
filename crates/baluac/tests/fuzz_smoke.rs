//! Fuzz smoke (M3): the frontend must never panic, on any input.
//!
//! Deterministic xorshift PRNG with a fixed seed — reproducible, no extra
//! dependencies. The full 1h soak is an M3-exit activity; this suite runs a
//! fast subset plus every in-repo `.bl` file as seed corpus. Panics fail the
//! test automatically; diagnostics are allowed.

use baluac_lib::lexer::Lexer;
use baluac_lib::parser::Parser;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n as u64)) as usize
    }
}

const PIECES: &[&str] = &[
    "fn", "let", "mut", "const", "if", "else", "for", "while", "loop", "match", "return",
    "break", "continue", "spawn", "chan", "send", "recv", "select", "unsafe", "extern",
    "pub", "priv", "as", "in", "main", "foo", "x", "i32", "u32", "bool",
    "{", "}", "(", ")", "[", "]", ";", ",", ":", "->", "=>", "::",
    "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "&&", "||", "&", "|", "^", "<<", ">>", "..", "=",
    "0", "1", "42", "3.14", "\"s\"", "true", "_",
    "@hw::gpu", "@hw::bogus", "#[max_stack(8)]", "// c", "\"unterminated",
];

fn no_panic(src: &str) {
    // Optional per-case dump for crash triage: the last dumped source is the
    // crasher (flushed before lexing, so it survives aborts that swallow
    // captured test output).
    if let Ok(path) = std::env::var("BALUA_FUZZ_DUMP") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
        let _ = writeln!(f, "===CASE===\n{}", src);
        let _ = f.flush();
    }
    let mut lexer = Lexer::new(src, "fuzz.bl");
    let (tokens, _) = lexer.tokenize();
    // The token stream must always be Eof-terminated for the parser.
    assert!(!tokens.is_empty(), "empty token stream for {:?}", src);
    let mut parser = Parser::new(tokens);
    let _ = parser.parse_program();
}

#[test]
fn fuzz_random_token_soup() {
    // Soak scale: `BALUA_FUZZ_CASES=N cargo test --test fuzz_smoke`
    // (default 3000). Same fixed seed, so larger runs extend the sequence.
    let cases: usize = std::env::var("BALUA_FUZZ_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let mut rng = Rng(0xBA10A); // fixed seed → reproducible
    // `BALUA_FUZZ_SKIP=N` fast-forwards the PRNG past N cases (same sequence).
    let skip: usize = std::env::var("BALUA_FUZZ_SKIP").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    for _ in 0..skip.min(cases) {
        let n = 1 + rng.below(40);
        for _ in 0..n {
            rng.below(PIECES.len());
        }
    }
    // Optional progress log (index every 1000 cases) for long soaks: the
    // trailing index after an abort bounds the crashing case.
    let log_every: usize = std::env::var("BALUA_FUZZ_LOG_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let log_path = std::env::var("BALUA_FUZZ_LOG").ok();
    for (i, _) in (0..cases).enumerate() {
        let n = 1 + rng.below(40);
        let mut src = String::new();
        for _ in 0..n {
            src.push_str(PIECES[rng.below(PIECES.len())]);
            src.push(' ');
        }
        if log_every > 0 && i % log_every == 0 {
            if let Some(ref path) = log_path {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
                let _ = writeln!(f, "case {}", skip + i);
                let _ = f.flush();
            }
        }
        no_panic(&src);
    }
    // Completion record for long partitioned soaks (survives output capture).
    if let Ok(path) = std::env::var("BALUA_FUZZ_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "DONE skip={} cases={} fails=0", skip, cases);
            let _ = f.flush();
        }
    }
}

#[test]
fn fuzz_seed_corpus_never_panics() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for dir in ["../../examples", "../../tests", "../../selfhost", "../../std", "../../std/hal", "../../std/ffi"] {
        let d = root.join(dir);
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("bl") {
                    files.push(p);
                }
            }
        }
    }
    files.push(root.join("../../hello.bl"));
    files.push(root.join("../../keyword_test.bl"));
    assert!(!files.is_empty(), "seed corpus empty");
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        no_panic(&src);
    }
}
