//! Balua Parser — Stage 2 (Section 2.1)
//! Recursive-descent parser producing typed AST with human-readable errors.

use crate::ast::*;
use crate::diagnostics::{Diagnostic, Span};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diags: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diags: Vec::new(),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn check(&self, lex: &str) -> bool {
        self.peek().lexeme == lex
    }

    fn check_kind(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn expect(&mut self, lex: &str) -> Result<Token, Diagnostic> {
        if self.check(lex) {
            Ok(self.advance())
        } else {
            let span = self.peek().span.clone();
            Err(Diagnostic::error(format!(
                "Expected '{}' but found '{}' at {}:{}",
                lex, self.peek().lexeme, span.line, span.col
            ))
            .with_span(span))
        }
    }

    fn at_eof(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    pub fn parse_program(&mut self) -> (Program, Vec<Diagnostic>) {
        let mut modules = Vec::new();
        let mut items = Vec::new();

        while !self.at_eof() {
            // skip comments
            if self.peek().kind == TokenKind::Comment {
                self.advance();
                continue;
            }
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(d) => {
                    self.diags.push(d);
                    self.synchronize();
                }
            }
        }

        if !items.is_empty() {
            modules.push(Module {
                name: "main".into(),
                items,
                span: dummy_span(),
            });
        }

        (
            Program { modules },
            std::mem::take(&mut self.diags),
        )
    }

    fn synchronize(&mut self) {
        while !self.at_eof() && !self.check(";") && !self.check("}") {
            self.advance();
            if self.check("fn") || self.check("let") || self.check("const") || self.check("struct") || self.check("enum") || self.check("match") || self.check("for") || self.check("while") || self.check("loop") || self.check("return") || self.check("break") || self.check("continue") || self.check("unsafe") {
                break;
            }
        }
        if self.check(";") || self.check("}") {
            self.advance();
        }
    }

    fn parse_item(&mut self) -> Result<Item, Diagnostic> {
        // hardware annotation prefix
        let hw = if self.peek().kind == TokenKind::HardwareDirective {
            Some(self.parse_hardware_annotation()?)
        } else {
            None
        };

        // annotations like #[...] — skip for now but preserve
        while self.peek().kind == TokenKind::Annotation {
            self.advance();
        }

        let tok = self.peek().clone();
        match tok.lexeme.as_str() {
            "fn" | "async" => {
                let mut decl = self.parse_fn_decl()?;
                if let Some(h) = hw {
                    decl.hardware = Some(h);
                }
                // classify kernel/circuit based on hardware target
                if let Some(hw) = &decl.hardware {
                    match hw.target {
                        HardwareTarget::Gpu => {
                            return Ok(Item::KernelDecl(KernelDecl {
                                name: decl.name.clone(),
                                params: decl.params.clone(),
                                annotation: hw.clone(),
                                body: decl.body.clone().unwrap_or(Block { stmts: vec![], span: dummy_span() }),
                                span: decl.span.clone(),
                            }))
                        }
                        HardwareTarget::Fpga => {
                            return Ok(Item::CircuitDecl(CircuitDecl {
                                name: decl.name.clone(),
                                params: decl.params.clone(),
                                ret_ty: decl.ret_ty.clone(),
                                annotation: hw.clone(),
                                body: decl.body.clone().unwrap_or(Block { stmts: vec![], span: dummy_span() }),
                                span: decl.span.clone(),
                            }))
                        }
                        HardwareTarget::Quantum => {
                            return Ok(Item::QuantumBlock(QuantumBlock {
                                annotation: hw.clone(),
                                qubits: 1,
                                body: decl.body.clone().unwrap_or(Block { stmts: vec![], span: dummy_span() }),
                                span: decl.span.clone(),
                            }))
                        }
                        _ => {}
                    }
                }
                Ok(Item::FnDecl(decl))
            }
            "kernel" => {
                // kernel fn ... sugar
                self.advance(); // kernel
                let mut decl = self.parse_fn_decl()?;
                if let Some(h) = hw {
                    decl.hardware = Some(h);
                }
                Ok(Item::KernelDecl(KernelDecl {
                    name: decl.name,
                    params: decl.params,
                    annotation: decl.hardware.unwrap_or(HardwareAnnotation { target: HardwareTarget::Gpu, params: vec![], span: dummy_span() }),
                    body: decl.body.unwrap_or(Block { stmts: vec![], span: dummy_span() }),
                    span: decl.span,
                }))
            }
            "circuit" => {
                self.advance();
                let mut decl = self.parse_fn_decl()?;
                if let Some(h) = hw {
                    decl.hardware = Some(h);
                }
                Ok(Item::CircuitDecl(CircuitDecl {
                    name: decl.name,
                    params: decl.params,
                    ret_ty: decl.ret_ty,
                    annotation: decl.hardware.unwrap_or(HardwareAnnotation { target: HardwareTarget::Fpga, params: vec![], span: dummy_span() }),
                    body: decl.body.unwrap_or(Block { stmts: vec![], span: dummy_span() }),
                    span: decl.span,
                }))
            }
            "let" => Ok(Item::VarDecl(self.parse_var_decl()?)),
            "const" => Ok(Item::ConstDecl(self.parse_const_decl()?)),
            "struct" => Ok(Item::StructDecl(self.parse_struct()?)),
            "enum" => Ok(Item::EnumDecl(self.parse_enum()?)),
            "trait" => Ok(Item::TraitDecl(self.parse_trait()?)),
            "impl" => Ok(Item::ImplDecl(self.parse_impl()?)),
            "use" | "module" => Ok(Item::UseDecl(self.parse_use()?)),
            "extern" => Ok(Item::FFIDecl(self.parse_ffi()?)),
            "unsafe" => Ok(Item::UnsafeBlock(self.parse_unsafe()?)),
            _ => {
                // If hardware block wraps items: @hw::... { ... }
                if let Some(h) = hw {
                    // expect block
                    let block = self.parse_block()?;
                    return Ok(Item::HardwareBlock(HardwareBlock {
                        annotation: h,
                        items: vec![], // block stmts not lowered to items in this minimal parser
                        span: block.span,
                    }));
                }
                Err(Diagnostic::error(format!("Unexpected token '{}'", tok.lexeme)).with_span(tok.span))
            }
        }
    }

    fn parse_hardware_annotation(&mut self) -> Result<HardwareAnnotation, Diagnostic> {
        let tok = self.advance();
        let lex = tok.lexeme.clone();
        let target = if lex.contains("gpu") {
            HardwareTarget::Gpu
        } else if lex.contains("fpga") {
            HardwareTarget::Fpga
        } else if lex.contains("npu") {
            HardwareTarget::Npu
        } else if lex.contains("quantum") {
            HardwareTarget::Quantum
        } else if lex.contains("cpu") {
            HardwareTarget::Cpu
        } else {
            HardwareTarget::Embedded
        };

        let mut params = Vec::new();
        // optional (key=val, ...)
        if self.check("(") {
            self.advance(); // (
            while !self.check(")") && !self.at_eof() {
                let k = self.advance().lexeme;
                if self.check("=") {
                    self.advance();
                    let v = self.advance().lexeme;
                    params.push((k, v));
                } else {
                    params.push((k, String::new()));
                }
                if self.check(",") {
                    self.advance();
                }
            }
            self.expect(")")?;
        }

        // Validate hardware target
        if !["@hw::gpu","@hw::fpga","@hw::npu","@hw::quantum","@hw::cpu","@hw::embedded"].iter().any(|d| lex.starts_with(d)) && lex.starts_with("@hw::") {
            return Err(Diagnostic::error(format!("Unknown hardware target '{}'", lex))
                .with_span(tok.span.clone())
                .with_hint("Valid: @hw::gpu, @hw::fpga, @hw::npu, @hw::quantum, @hw::cpu, @hw::embedded"));
        }

        Ok(HardwareAnnotation { target, params, span: tok.span })
    }

    fn parse_fn_decl(&mut self) -> Result<FnDecl, Diagnostic> {
        let start = self.peek().span.clone();
        let is_async = if self.check("async") { self.advance(); true } else { false };
        let is_extern = if self.check("extern") {
            self.advance();
            // optional ABI string
            let abi = if self.peek().kind == TokenKind::LiteralStr { self.advance().lexeme } else { "C".into() };
            Some(abi)
        } else { None };

        self.expect("fn")?;
        let visibility = if self.check("pub") { self.advance(); Visibility::Pub }
            else if self.check("priv") { self.advance(); Visibility::Priv }
            else { Visibility::Default };
        let name = self.advance().lexeme; // ident
        // generics <...>
        let generics = if self.check("<") {
            self.advance();
            let mut g = Vec::new();
            while !self.check(">") && !self.at_eof() {
                g.push(self.advance().lexeme);
                if self.check(",") { self.advance(); }
            }
            self.expect(">")?;
            g
        } else { vec![] };
        // where clause
        let where_clause = if self.check("where") {
            self.advance();
            let mut w = Vec::new();
            while !self.check("{") && !self.check(";") && !self.at_eof() {
                let ty = self.advance().lexeme;
                let bound = if self.check(":") { self.advance(); self.advance().lexeme } else { String::new() };
                w.push((ty, bound));
                if self.check(",") { self.advance(); }
            }
            w
        } else { vec![] };

        self.expect("(")?;
        let mut params = Vec::new();
        while !self.check(")") && !self.at_eof() {
            if self.check(",") { self.advance(); continue; }
            let is_mut = if self.check("mut") { self.advance(); true } else { false };
            let n = self.advance().lexeme;
            self.expect(":")?;
            let ty = self.parse_type()?;
            params.push(Param { name: n, ty, is_mut });
            if self.check(",") { self.advance(); }
        }
        self.expect(")")?;
        let ret_ty = if self.check("->") {
            self.advance();
            Some(self.parse_type()?)
        } else { None };

        // optional hardware annotation after signature: @hw::gpu
        let hardware = if self.peek().kind == TokenKind::HardwareDirective {
            Some(self.parse_hardware_annotation()?)
        } else { None };

        let body = if self.check("{") { Some(self.parse_block()?) } else if self.check(";") { self.advance(); None } else { None };

        Ok(FnDecl { name, generics, where_clause, params, ret_ty, hardware, is_async, is_extern, visibility, body, span: start })
    }

    fn parse_type(&mut self) -> Result<TypeExpr, Diagnostic> {
        // Handle references, pointers, generics
        if self.check("&") {
            self.advance();
            let is_mut = if self.check("mut") { self.advance(); true } else { false };
            // lifetime?
            let lt = if self.check("'") { Some(self.advance().lexeme) } else if self.peek().lexeme.starts_with('\'') { Some(self.advance().lexeme) } else { None };
            let inner = Box::new(self.parse_type()?);
            return Ok(TypeExpr::Reference { is_mut, lifetime: lt, inner });
        }
        if self.check("*") {
            self.advance();
            let is_mut = if self.check("mut") { self.advance(); true } else { false };
            if !is_mut && self.check("const") { self.advance(); }
            let inner = Box::new(self.parse_type()?);
            return Ok(TypeExpr::Pointer { is_mut, inner });
        }
        let base = self.advance().lexeme;
        if base == "qubit" {
            if self.check("[") {
                self.advance();
                let n: usize = self.advance().lexeme.parse().unwrap_or(1);
                self.expect("]")?;
                return Ok(TypeExpr::Qubit(Some(n)));
            }
            return Ok(TypeExpr::Qubit(None));
        }
        // generic args <T, ...> or [T; N]
        if self.check("<") {
            self.advance();
            let mut args = Vec::new();
            while !self.check(">") && !self.at_eof() {
                args.push(self.parse_type()?);
                if self.check(",") { self.advance(); }
            }
            self.expect(">")?;
            return Ok(TypeExpr::Generic { name: base, args });
        }
        if self.check("[") {
            self.advance();
            let inner = Box::new(TypeExpr::Primitive(base.clone()));
            // size or shape?
            if self.check("]") { self.advance(); return Ok(TypeExpr::Slice(inner)); }
            let sz: usize = self.advance().lexeme.parse().unwrap_or(0);
            self.expect("]")?;
            return Ok(TypeExpr::Array { inner, size: sz });
        }
        Ok(TypeExpr::Primitive(base))
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("let")?;
        let is_mut = if self.check("mut") { self.advance(); true } else { false };
        let name = self.advance().lexeme;
        let ty = if self.check(":") { self.advance(); Some(self.parse_type()?) } else { None };
        let init = if self.check("=") {
            self.advance();
            Some(self.parse_expr()?)
        } else { None };
        if self.check(";") { self.advance(); }
        Ok(VarDecl { name, ty, init, is_mut, ownership: Ownership::Owned, span })
    }

    fn parse_const_decl(&mut self) -> Result<ConstDecl, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("const")?;
        let visibility = if self.check("pub") { self.advance(); Visibility::Pub }
            else if self.check("priv") { self.advance(); Visibility::Priv }
            else { Visibility::Default };
        let name = self.advance().lexeme;
        let ty = if self.check(":") { self.advance(); Some(self.parse_type()?) } else { None };
        self.expect("=")?;
        let value = self.parse_expr()?;
        if self.check(";") { self.advance(); }
        Ok(ConstDecl { name, ty, value, visibility, span })
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        // very minimal: ident / literal / call / binary / cast
        let tok = self.advance();
        let mut expr = match tok.kind {
            TokenKind::LiteralInt => Expr::Literal(Literal::Int(tok.lexeme.parse().unwrap_or(0), tok.lexeme)),
            TokenKind::LiteralFloat => Expr::Literal(Literal::Float(tok.lexeme.parse().unwrap_or(0.0), tok.lexeme)),
            TokenKind::LiteralStr => Expr::Literal(Literal::Str(tok.lexeme)),
            TokenKind::LiteralBool => Expr::Literal(Literal::Bool(tok.lexeme == "true")),
            TokenKind::Identifier => Expr::Ident(tok.lexeme),
            _ if tok.lexeme == "&" => {
                let is_mut = if self.check("mut") { self.advance(); true } else { false };
                let inner = Box::new(self.parse_expr()?);
                Expr::BorrowExpr { inner, is_mut, lifetime: None }
            }
            _ => Expr::Ident(tok.lexeme),
        };
        // handle call or binary op
        if self.check("(") {
            self.advance();
            let mut args = Vec::new();
            while !self.check(")") && !self.at_eof() {
                args.push(self.parse_expr()?);
                if self.check(",") { self.advance(); }
            }
            self.expect(")")?;
            expr = Expr::Call { callee: Box::new(expr), args };
        }
        // cast: expr as Type
        if self.check("as") {
            self.advance();
            let ty = self.parse_type()?;
            expr = Expr::Cast { expr: Box::new(expr), ty };
        }
        if self.peek().kind == TokenKind::Operator {
            let op = self.advance().lexeme;
            let rhs = self.parse_expr()?;
            expr = Expr::Binary { op, lhs: Box::new(expr), rhs: Box::new(rhs) };
        }
        Ok(expr)
    }

    fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("{")?;
        let mut stmts = Vec::new();
        while !self.check("}") && !self.at_eof() {
            if self.peek().kind == TokenKind::Comment { self.advance(); continue; }
            if self.check("let") {
                stmts.push(Stmt::Let(self.parse_var_decl()?));
            } else if self.check("const") {
                stmts.push(Stmt::Const(self.parse_const_decl()?));
            } else if self.check("return") {
                self.advance();
                let e = if !self.check(";") && !self.check("}") { Some(self.parse_expr()?) } else { None };
                if self.check(";") { self.advance(); }
                stmts.push(Stmt::Return(e));
            } else if self.check("match") {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect("{")?;
                let mut arms = Vec::new();
                while !self.check("}") && !self.at_eof() {
                    let pat = self.advance().lexeme;
                    self.expect("=>")?;
                    let arm_expr = self.parse_expr()?;
                    arms.push(MatchArm { pattern: pat, expr: arm_expr });
                    if self.check(",") { self.advance(); }
                }
                self.expect("}")?;
                stmts.push(Stmt::Match { expr, arms });
            } else if self.check("for") {
                self.advance();
                let var = self.advance().lexeme;
                self.expect("in")?;
                let iter = self.parse_expr()?;
                let body = self.parse_block()?;
                stmts.push(Stmt::For { var, iter, body });
            } else if self.check("while") {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                stmts.push(Stmt::While { cond, body });
            } else if self.check("loop") {
                self.advance();
                let body = self.parse_block()?;
                stmts.push(Stmt::Loop { body });
            } else if self.check("break") {
                self.advance();
                let e = if !self.check(";") && !self.check("}") { Some(self.parse_expr()?) } else { None };
                if self.check(";") { self.advance(); }
                stmts.push(Stmt::Break(e));
            } else if self.check("continue") {
                self.advance();
                if self.check(";") { self.advance(); }
                stmts.push(Stmt::Continue);
            } else if self.check("if") {
                self.advance();
                let cond = self.parse_expr()?;
                let then_block = self.parse_block()?;
                let else_block = if self.check("else") { self.advance(); Some(Box::new(Expr::Block(self.parse_block()?))) } else { None };
                stmts.push(Stmt::Expr(Expr::If { cond: Box::new(cond), then_block: Box::new(then_block), else_block }));
            } else {
                let e = self.parse_expr()?;
                if self.check(";") { self.advance(); }
                stmts.push(Stmt::Expr(e));
            }
        }
        self.expect("}")?;
        Ok(Block { stmts, span })
    }

    fn parse_struct(&mut self) -> Result<StructDecl, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("struct")?;
        let name = self.advance().lexeme;
        let generics = vec![];
        self.expect("{")?;
        let mut fields = Vec::new();
        while !self.check("}") && !self.at_eof() {
            let fname = self.advance().lexeme;
            self.expect(":")?;
            let ty = self.parse_type()?;
            fields.push(Field { name: fname, ty });
            if self.check(",") { self.advance(); }
        }
        self.expect("}")?;
        Ok(StructDecl { name, generics, fields, span })
    }

    fn parse_enum(&mut self) -> Result<EnumDecl, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("enum")?;
        let name = self.advance().lexeme;
        self.expect("{")?;
        let mut variants = Vec::new();
        while !self.check("}") && !self.at_eof() {
            let vname = self.advance().lexeme;
            let fields = if self.check("(") {
                self.advance();
                let mut f = Vec::new();
                while !self.check(")") { f.push(self.parse_type()?); if self.check(",") { self.advance(); } }
                self.expect(")")?;
                f
            } else { vec![] };
            variants.push(EnumVariant { name: vname, fields });
            if self.check(",") { self.advance(); }
        }
        self.expect("}")?;
        Ok(EnumDecl { name, variants, span })
    }

    fn parse_trait(&mut self) -> Result<TraitDecl, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("trait")?;
        let name = self.advance().lexeme;
        self.expect("{")?;
        let mut methods = Vec::new();
        while !self.check("}") { methods.push(self.parse_fn_decl()?); }
        self.expect("}")?;
        Ok(TraitDecl { name, methods, span })
    }

    fn parse_impl(&mut self) -> Result<ImplDecl, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("impl")?;
        let first = self.advance().lexeme;
        let (trait_name, target) = if self.check("for") { self.advance(); let t = self.advance().lexeme; (Some(first), t) } else { (None, first) };
        self.expect("{")?;
        let mut methods = Vec::new();
        while !self.check("}") { methods.push(self.parse_fn_decl()?); }
        self.expect("}")?;
        Ok(ImplDecl { trait_name, target, methods, span })
    }

    fn parse_use(&mut self) -> Result<UseDecl, Diagnostic> {
        let span = self.peek().span.clone();
        if self.check("use") { self.advance(); } else { self.expect("module")?; }
        let mut path = String::new();
        while !self.check(";") && !self.at_eof() {
            path.push_str(&self.advance().lexeme);
        }
        if self.check(";") { self.advance(); }
        Ok(UseDecl { path, span })
    }

    fn parse_ffi(&mut self) -> Result<FFIDecl, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("extern")?;
        let abi = if self.peek().kind == TokenKind::LiteralStr { self.advance().lexeme } else { self.advance().lexeme };
        self.expect("fn")?;
        let name = self.advance().lexeme;
        self.expect("(")?;
        let mut params = Vec::new();
        while !self.check(")") { let n = self.advance().lexeme; self.expect(":")?; let ty = self.parse_type()?; params.push(Param { name: n, ty, is_mut: false }); if self.check(",") { self.advance(); } }
        self.expect(")")?;
        let ret_ty = if self.check("->") { self.advance(); Some(self.parse_type()?) } else { None };
        if self.check(";") { self.advance(); }
        Ok(FFIDecl { abi, name, params, ret_ty, span })
    }

    fn parse_unsafe(&mut self) -> Result<UnsafeBlock, Diagnostic> {
        let span = self.peek().span.clone();
        self.expect("unsafe")?;
        let b = self.parse_block()?;
        Ok(UnsafeBlock { stmts: b.stmts, span })
    }
}

fn dummy_span() -> Span {
    Span { file: "<dummy>".into(), line: 1, col: 1, end_line: 1, end_col: 1 }
}
