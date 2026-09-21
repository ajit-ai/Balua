# v0.5 Self-Host Prep

Goal: balua.exe builds balua components written in Balua (selfhost/*.bl) — first step to baluac-in-Balua (v1.0).

- selfhost/lexer.bl — mirrors crates/baluac/src/lexer.rs (TokenKind, Span, HardwareDirective)
- Next: selfhost/parser.bl (mirrors parser.rs), selfhost/ast.bl

Validate:
  balua selfhost/lexer.bl --emit-mir
  BPM build --target x86_64  # via baluac LLVM thin LTO (.cargo/config.toml)
