use baluac_lib::lexer::{Lexer, TokenKind};

#[test]
fn tokenizes_hello_hardware() {
    let src = r#"fn main() -> i32 { println("Hello from Balua!"); }"#;
    let mut l = Lexer::new(src, "test.bl");
    let (toks, diags) = l.tokenize();
    assert!(diags.is_empty());
    assert!(toks.iter().any(|t| t.lexeme=="fn"));
}

#[test]
fn hardware_directive_is_first_class() {
    let src = r#"@hw::gpu(backend=cuda, sm=90) kernel fn mm() {}"#;
    let mut l = Lexer::new(src, "test.bl");
    let (toks, _) = l.tokenize();
    assert_eq!(toks[0].kind, TokenKind::HardwareDirective);
}
