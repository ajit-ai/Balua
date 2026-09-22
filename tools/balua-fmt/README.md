# balua-fmt

Token-driven formatter: re-emits the lexer token stream with canonical
spacing and 4-space indentation, preserving `@hw::` directives and `#[...]`
annotations. Whitespace-only — never reorders or drops tokens; idempotent
and token-preserving (tested). Normalizes `} else {` across lines, moves
trailing comments to their own line, and ensures a trailing newline.

Usage: `cargo run -p balua-fmt -- [--check] [--write] <files-or-dirs...>`
(default prints to stdout).

