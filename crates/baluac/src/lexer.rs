//! Balua Lexer — Stage 1 of Compiler Frontend (Section 2.1)
//! Tokenizes .bl source into token stream.
//! Supports UTF-8 identifiers, @hw:: hardware directives, annotations.

use crate::diagnostics::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Identifier,
    LiteralInt,
    LiteralFloat,
    LiteralStr,
    LiteralBool,
    Operator,
    Delimiter,
    Comment,
    HardwareDirective, // @hw::gpu , @hw::fpga etc.
    Annotation,        // #[...] or #pragma
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            span,
        }
    }
}

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "const", "struct", "enum", "trait", "impl", "mod", "module", "use",
    "if", "else", "for", "while", "loop", "match", "return", "break", "continue", "unsafe",
    "async", "await", "spawn", "chan", "send", "recv", "select", "extern", "type", "where", "pub", "priv",
    "self", "Self", "super", "crate", "box", "move", "in", "as", "is", "kernel", "circuit",
    "tensor", "qubit", "true", "false",
];

const HARDWARE_DIRECTIVES: &[&str] = &[
    "@hw::gpu",
    "@hw::fpga",
    "@hw::npu",
    "@hw::quantum",
    "@hw::cpu",
    "@hw::embedded",
];

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    file: String,
}

impl Lexer {
    pub fn new(src: &str, file: impl Into<String>) -> Self {
        Self {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            file: file.into(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        let mut diags = Vec::new();

        loop {
            self.skip_whitespace();
            let start_line = self.line;
            let start_col = self.col;
            let file = self.file.clone();
            let mk_span = |s_line, s_col, e_line, e_col, f: String| Span {
                file: f,
                line: s_line,
                col: s_col,
                end_line: e_line,
                end_col: e_col,
            };

            match self.peek() {
                None => {
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(TokenKind::Eof, "", span));
                    break;
                }
                Some('/') if self.peek2() == Some('/') => {
                    let mut lex = String::new();
                    while let Some(c) = self.peek() {
                        if c == '\n' { break; }
                        lex.push(c);
                        self.advance();
                    }
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(TokenKind::Comment, lex, span));
                }
                Some('/') if self.peek2() == Some('*') => {
                    let mut lex = String::from("/*");
                    self.advance(); self.advance();
                    let mut closed = false;
                    while let Some(c) = self.peek() {
                        self.advance();
                        lex.push(c);
                        if c == '*' && self.peek() == Some('/') {
                            lex.push('/'); self.advance(); closed = true; break;
                        }
                    }
                    if !closed {
                        let span = mk_span(start_line, start_col, self.line, self.col, file.clone());
                        diags.push(Diagnostic::error("Unterminated block comment").with_span(span));
                    }
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(TokenKind::Comment, lex, span));
                }
                Some('@') => {
                    let mut lex = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_whitespace() || c == '(' || c == '\n' { break; }
                        lex.push(c); self.advance();
                        if lex.starts_with("@hw::") && lex.len() > 5 {
                            if let Some(nc) = self.peek() {
                                if !(nc.is_alphanumeric() || nc == '_' || nc == ':') { break; }
                            }
                        }
                    }
                    let kind = if HARDWARE_DIRECTIVES.iter().any(|d| lex.starts_with(d)) {
                        TokenKind::HardwareDirective
                    } else if lex.starts_with('@') { TokenKind::HardwareDirective } else { TokenKind::Annotation };
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(kind, lex, span));
                }
                Some('#') => {
                    let mut lex = String::new();
                    while let Some(c) = self.peek() {
                        lex.push(c); self.advance();
                        if c == ']' { break; }
                        if c == '\n' { break; }
                        if lex == "#pragma" {
                            while let Some(cc) = self.peek() {
                                if cc == '\n' { break; }
                                lex.push(cc); self.advance();
                            }
                            break;
                        }
                        if lex.starts_with("#[") && c == ']' { break; }
                    }
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(TokenKind::Annotation, lex, span));
                }
                Some('"') | Some('\'') => {
                    let quote = self.advance().unwrap();
                    let mut lex = String::new(); lex.push(quote);
                    let mut escaped = false; let mut closed = false;
                    while let Some(c) = self.peek() {
                        self.advance(); lex.push(c);
                        if escaped { escaped = false; }
                        else if c == '\\' { escaped = true; }
                        else if c == quote { closed = true; break; }
                        else if c == '\n' { break; }
                    }
                    if !closed {
                        let span = mk_span(start_line, start_col, self.line, self.col, file.clone());
                        diags.push(Diagnostic::error("Unterminated string literal").with_span(span));
                    }
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(TokenKind::LiteralStr, lex, span));
                }
                Some(c) if c.is_ascii_digit() => {
                    let mut lex = String::new(); let mut is_float = false;
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_digit() || ch == '_' { lex.push(ch); self.advance(); }
                        else if ch == '.' && self.peek2().map(|n| n.is_ascii_digit()).unwrap_or(false) { is_float = true; lex.push(ch); self.advance(); }
                        else if ch == 'x' || ch == 'b' || ch == 'o' { lex.push(ch); self.advance(); }
                        else if ch.is_ascii_alphabetic() { lex.push(ch); self.advance(); }
                        else { break; }
                    }
                    let kind = if is_float || lex.contains("f16") || lex.contains("f32") || lex.contains("f64") { TokenKind::LiteralFloat } else { TokenKind::LiteralInt };
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(kind, lex, span));
                }
                Some(c) if is_ident_start(c) => {
                    let mut lex = String::new();
                    while let Some(ch) = self.peek() {
                        if is_ident_continue(ch) { lex.push(ch); self.advance(); } else { break; }
                    }
                    let kind = if lex == "true" || lex == "false" { TokenKind::LiteralBool }
                    else if KEYWORDS.contains(&lex.as_str()) { TokenKind::Keyword } else { TokenKind::Identifier };
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(kind, lex, span));
                }
                Some(c) => {
                    self.advance();
                    let mut lex = String::from(c);
                    if let Some(nc) = self.peek() {
                        let two = format!("{}{}", c, nc);
                        if ["==", "!=", "<=", ">=", "->", "=>", "::", "&&", "||", "<<", ">>", "+=", "-=", "*=", "/=", ".."].contains(&two.as_str()) {
                            lex.push(nc); self.advance();
                        }
                    }
                    let kind = match lex.as_str() {
                        "{" | "}" | "(" | ")" | "[" | "]" | ";" | "," | ":" => TokenKind::Delimiter,
                        "::" | "->" | "=>" => TokenKind::Operator,
                        _ if "{}()[];:, ".contains(lex.chars().next().unwrap()) => TokenKind::Delimiter,
                        _ if "{}()[]".contains(c) => TokenKind::Delimiter,
                        _ => if ";,".contains(c) { TokenKind::Delimiter } else { TokenKind::Operator },
                    };
                    let span = mk_span(start_line, start_col, self.line, self.col, file);
                    tokens.push(Token::new(kind, lex, span));
                }
            }
        }

        (tokens, diags)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || (c as u32) > 127
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || (c as u32) > 127
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lex_hardware_directive() {
        let mut l = Lexer::new("@hw::gpu(backend=cuda)", "test.bl");
        let (toks, _) = l.tokenize();
        assert_eq!(toks[0].kind, TokenKind::HardwareDirective);
        assert!(toks[0].lexeme.starts_with("@hw::gpu"));
    }
    #[test]
    fn lex_unicode_ident() {
        let mut l = Lexer::new("let café = 42;", "test.bl");
        let (toks, _) = l.tokenize();
        let id = toks.iter().find(|t| t.lexeme == "café").unwrap();
        assert_eq!(id.kind, TokenKind::Identifier);
    }
}
