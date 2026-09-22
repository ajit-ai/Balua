//! balua-fmt — Balua code formatter (M3).
//!
//! Token-driven and whitespace-only: it re-emits the lexer token stream with
//! canonical spacing and 4-space indentation, never reordering or dropping
//! tokens. `@hw::` directives and `#[...]` annotations are preserved. Single
//! blank lines between items are kept; `};` stays joined. Known
//! normalizations: `} else {` splits across lines, trailing comments move to
//! their own line, unary `-` prints spaced (`- 5`), and output uses LF line
//! endings (see `.gitattributes` for `*.bl`).
//!
//! Usage: `balua-fmt [--check] [--write] <files-or-dirs...>` (default: stdout).
//! Directories contribute their `*.bl` files (non-recursive). Exit 1 on
//! differences in `--check` mode or on I/O errors.

use baluac_lib::lexer::{Lexer, Token, TokenKind};

fn is_word(t: &Token) -> bool {
    matches!(
        t.kind,
        TokenKind::Keyword
            | TokenKind::Identifier
            | TokenKind::LiteralInt
            | TokenKind::LiteralFloat
            | TokenKind::LiteralStr
            | TokenKind::LiteralBool
    )
}

/// Format Balua source text. `file` is used for diagnostic spans only.
pub fn format(src: &str, file: &str) -> String {
    let mut lexer = Lexer::new(src, file);
    let (tokens, _) = lexer.tokenize();
    let toks: Vec<Token> = tokens.into_iter().filter(|t| t.kind != TokenKind::Eof).collect();
    let mut out = String::new();
    let mut indent: usize = 0;
    let mut at_line_start = true;
    // True when the next word token needs a leading space.
    let mut need_space = false;
    // True when the previous token was `}` (for `};` joining).
    let mut prev_rbrace = false;
    // End line of the previous token (blank-line preservation).
    let mut prev_end_line: usize = 0;

    let do_indent = |out: &mut String, indent: usize, at_line_start: &mut bool| {
        if *at_line_start {
            for _ in 0..indent {
                out.push_str("    ");
            }
            *at_line_start = false;
        }
    };

    for tok in &toks {
        // Preserve a single blank line between items (never at file start).
        if at_line_start && !out.is_empty() && tok.span.line > prev_end_line + 1 {
            out.push('\n');
        }
        let was_rbrace = prev_rbrace;
        prev_rbrace = false;
        match tok.kind {
            TokenKind::Comment | TokenKind::Annotation => {
                if !at_line_start {
                    out.push('\n');
                }
                do_indent(&mut out, indent, &mut at_line_start);
                out.push_str(&tok.lexeme);
                out.push('\n');
                at_line_start = true;
                need_space = false;
            }
            TokenKind::HardwareDirective => {
                do_indent(&mut out, indent, &mut at_line_start);
                if need_space {
                    out.push(' ');
                }
                out.push_str(&tok.lexeme);
                need_space = true;
            }
            TokenKind::Delimiter => match tok.lexeme.as_str() {
                "(" | "[" => {
                    do_indent(&mut out, indent, &mut at_line_start);
                    out.push_str(&tok.lexeme);
                    need_space = false;
                }
                ")" | "]" => {
                    do_indent(&mut out, indent, &mut at_line_start);
                    out.push_str(&tok.lexeme);
                    need_space = true;
                }
                "{" => {
                    if !at_line_start {
                        if need_space {
                            out.push(' ');
                        }
                    } else {
                        do_indent(&mut out, indent, &mut at_line_start);
                    }
                    out.push('{');
                    out.push('\n');
                    indent += 1;
                    at_line_start = true;
                    need_space = false;
                }
                "}" => {
                    indent = indent.saturating_sub(1);
                    if !at_line_start {
                        out.push('\n');
                    }
                    do_indent(&mut out, indent, &mut at_line_start);
                    out.push('}');
                    out.push('\n');
                    at_line_start = true;
                    need_space = false;
                    prev_rbrace = true;
                }
                ";" => {
                    if was_rbrace {
                        // Join `};`: undo the newline `}` just emitted.
                        if out.ends_with('\n') {
                            out.pop();
                        }
                        out.push(';');
                    } else {
                        do_indent(&mut out, indent, &mut at_line_start);
                        out.push(';');
                    }
                    out.push('\n');
                    at_line_start = true;
                    need_space = false;
                }
                "," => {
                    do_indent(&mut out, indent, &mut at_line_start);
                    out.push_str(", ");
                    need_space = false;
                }
                ":" => {
                    do_indent(&mut out, indent, &mut at_line_start);
                    out.push_str(": ");
                    need_space = false;
                }
                _ => {
                    do_indent(&mut out, indent, &mut at_line_start);
                    out.push_str(&tok.lexeme);
                    need_space = true;
                }
            },
            TokenKind::Operator => match tok.lexeme.as_str() {
                // Tight operators keep their operands adjacent.
                "::" | ".." => {
                    do_indent(&mut out, indent, &mut at_line_start);
                    out.push_str(&tok.lexeme);
                    need_space = false;
                }
                _ => {
                    do_indent(&mut out, indent, &mut at_line_start);
                    if need_space {
                        out.push(' ');
                    }
                    out.push_str(&tok.lexeme);
                    out.push(' ');
                    need_space = false;
                }
            },
            _ => {
                // Keywords, identifiers, literals.
                debug_assert!(is_word(tok));
                do_indent(&mut out, indent, &mut at_line_start);
                if need_space {
                    out.push(' ');
                }
                out.push_str(&tok.lexeme);
                need_space = true;
            }
        }
        prev_end_line = tok.span.end_line;
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn print_usage() {
    eprintln!("usage: balua-fmt [--check] [--write] <files-or-dirs...>");
    eprintln!("  default: print formatted source to stdout");
    eprintln!("  --check: exit 1 and list files whose formatting differs");
    eprintln!("  --write: rewrite files in place");
}

fn collect(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        let p = std::path::Path::new(arg);
        if p.is_dir() {
            if let Ok(rd) = std::fs::read_dir(p) {
                let mut names: Vec<String> = rd
                    .flatten()
                    .map(|e| e.path())
                    .filter(|q| q.extension().and_then(|x| x.to_str()) == Some("bl"))
                    .map(|q| q.display().to_string())
                    .collect();
                names.sort();
                out.extend(names);
            }
        } else {
            out.push(arg.clone());
        }
    }
    out
}

fn main() {
    let mut check = false;
    let mut write = false;
    let mut files: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--check" => check = true,
            "--write" => write = true,
            "-h" | "--help" => {
                print_usage();
                return;
            }
            _ => files.push(arg),
        }
    }
    if files.is_empty() {
        print_usage();
        std::process::exit(2);
    }
    let files = collect(&files);
    let mut failed = false;
    for path in &files {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("balua-fmt: cannot read {}: {}", path, e);
                failed = true;
                continue;
            }
        };
        let formatted = format(&src, path);
        if check {
            if formatted != src {
                println!("would reformat: {}", path);
                failed = true;
            }
        } else if write {
            if formatted != src {
                if let Err(e) = std::fs::write(path, &formatted) {
                    eprintln!("balua-fmt: cannot write {}: {}", path, e);
                    failed = true;
                }
            }
        } else {
            print!("{}", formatted);
        }
    }
    if failed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexemes(src: &str) -> Vec<String> {
        let mut l = Lexer::new(src, "test.bl");
        let (toks, _) = l.tokenize();
        toks.into_iter().filter(|t| t.kind != TokenKind::Eof).map(|t| t.lexeme).collect()
    }

    #[test]
    fn braces_indent() {
        assert_eq!(format("fn main() -> i32 { 7 }", "t.bl"), "fn main() -> i32 {\n    7\n}\n");
    }

    #[test]
    fn semicolon_joins_rbrace() {
        assert_eq!(
            format("fn f() -> i32 { let mut r = 1; while n > 1 { r = r * n; }; r }", "t.bl"),
            "fn f() -> i32 {\n    let mut r = 1;\n    while n > 1 {\n        r = r * n;\n    };\n    r\n}\n"
        );
    }

    #[test]
    fn single_blank_lines_kept() {
        let src = "fn a() -> i32 { 0 }\n\n\nfn b() -> i32 { 1 }\n";
        assert_eq!(format(src, "t.bl"), "fn a() -> i32 {\n    0\n}\n\nfn b() -> i32 {\n    1\n}\n");
    }

    #[test]
    fn idempotent_on_snippets() {
        let cases = [
            "fn main() -> i32 { 7 }",
            "fn add(a: i32,b: i32)->i32{a+b}",
            "module main{use std::hal::cpu;fn main()->i32{let x=6*7;x}}",
            "@hw::gpu(backend=cuda, sm=90)\nkernel fn mm() -> void {\n}\n",
            "#[max_stack(512)]\nfn f() -> i32 { 0 }\n",
            "// comment\nfn m() -> i32 { let r = match x { 1 => 10, _ => 0 }; r }\n",
            "fn f() -> i32 { for i in 0..10 { if i % 2 == 0 { continue } } 0 }\n",
        ];
        for src in cases {
            let once = format(src, "t.bl");
            let twice = format(&once, "t.bl");
            assert_eq!(once, twice, "not idempotent for {:?}", src);
        }
    }

    #[test]
    fn preserves_token_stream() {
        let cases = [
            "fn main() -> i32 { let x = 6; let y = 7; x * y }",
            "module main{use std::hal::cpu;fn main()->i32{cpu::core_count()}}",
        ];
        for src in cases {
            assert_eq!(lexemes(src), lexemes(&format(src, "t.bl")), "tokens changed for {:?}", src);
        }
    }

    /// Dogfood: every in-repo `.bl` file is a fixed point of the formatter
    /// with an unchanged token stream.
    #[test]
    fn repo_corpus_stable() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = vec![root.join("hello.bl"), root.join("keyword_test.bl")];
        for dir in ["examples", "tests", "tests/hal", "selfhost", "std", "std/hal", "std/ffi"] {
            if let Ok(rd) = std::fs::read_dir(root.join(dir)) {
                let mut names: Vec<std::path::PathBuf> = rd
                    .flatten()
                    .map(|e| e.path())
                    .filter(|q| q.extension().and_then(|x| x.to_str()) == Some("bl"))
                    .collect();
                names.sort();
                files.extend(names);
            }
        }
        assert!(!files.is_empty(), "corpus empty");
        for path in &files {
            let src = std::fs::read_to_string(path).unwrap();
            let once = format(&src, &path.display().to_string());
            assert_eq!(once, src, "not formatted: {}", path.display());
            assert_eq!(lexemes(&src), lexemes(&once), "tokens changed: {}", path.display());
        }
    }
}
