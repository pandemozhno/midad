use crate::ast::*;
use crate::error::{Error, Result};
use crate::token::{Keyword, Token};

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    line: usize,
    col: usize,
    no_cast_as: bool,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Self { toks, pos: 0, line: 1, col: 0, no_cast_as: false }
    }

    pub fn parse_module(&mut self) -> Result<Module> {
        self.skip_newlines();
        let profile = self.parse_profile()?;
        self.skip_newlines();
        self.expect_kw(Keyword::Module)?;
        let name = self.expect_ident()?;
        self.expect(Token::Newline)?;

        let mut no_standard = false;
        let mut entry = None;
        let mut items = Vec::new();

        self.skip_newlines();
        if self.matches(Token::Indent) {
            self.advance();
            loop {
                match self.peek().cloned() {
                    Some(Token::Dedent) => { self.advance(); break; }
                    Some(Token::Newline) => { self.advance(); continue; }
                    Some(Token::Kw(Keyword::No)) => {
                        self.advance();
                        self.expect_kw(Keyword::Standard)?;
                        no_standard = true;
                        self.skip_newlines();
                    }
                    Some(Token::Kw(Keyword::Entry)) => {
                        self.advance();
                        entry = Some(self.expect_ident()?);
                        self.skip_newlines();
                    }
                    Some(Token::Eof) | None => break,
                    other => return Err(Error::parser(
                        format!("неожиданно в блоке директив module: {:?}", other),
                        self.line, self.col)),
                }
            }
        }

        loop {
            self.skip_newlines();
            if matches!(self.peek(), Some(Token::Eof) | None) {
                break;
            }
            items.push(self.parse_item()?);
        }

        Ok(Module { name, profile, no_standard, entry, items })
    }

    fn parse_profile(&mut self) -> Result<Profile> {
        let id = match self.advance().cloned() {
            Some(Token::Ident(s)) if s.starts_with('@') => s,
            Some(Token::Ident(s)) => return Err(Error::parser(
                format!("ожидался профиль @kernel/@flow/@view/@light, получено '{}'", s),
                self.line, self.col)),
            other => return Err(Error::parser(
                format!("ожидался профиль, получено {:?}", other), self.line, self.col)),
        };
        match id.as_str() {
            "@kernel" => Ok(Profile::Kernel),
            "@flow"   => Ok(Profile::Flow),
            "@view"   => Ok(Profile::View),
            "@light"  => Ok(Profile::Light),
            _ => Err(Error::parser(
                format!("неизвестный профиль '{}'", id), self.line, self.col)),
        }
    }

    fn parse_item(&mut self) -> Result<Item> {
        match self.peek().cloned() {
            Some(Token::Kw(Keyword::Unsafe))         => { self.advance(); self.parse_function(true) }
            Some(Token::Kw(Keyword::Function))       => self.parse_function(false),
            Some(Token::Kw(Keyword::Structure))      => self.parse_structure(),
            Some(Token::Kw(Keyword::Enumeration))    => self.parse_enumeration(),
            Some(Token::Kw(Keyword::Protocol))       => self.parse_protocol(),
            Some(Token::Kw(Keyword::Implementation)) => self.parse_implementation(),
            Some(Token::Kw(Keyword::Static))         => self.parse_static(),
            Some(Token::Kw(Keyword::External))       => self.parse_external(),
            Some(Token::Kw(Keyword::Rule))           => self.parse_rule(),
            other => Err(Error::parser(
                format!("ожидалось объявление, получено {:?}", other),
                self.line, self.col)),
        }
    }

    fn parse_function(&mut self, is_unsafe: bool) -> Result<Item> {
        self.expect_kw(Keyword::Function)?;
        let name = self.expect_ident()?;
        let (params, is_method) = self.parse_params()?;
        let ret = if self.matches_kw(Keyword::Gives) {
            self.advance();
            Some(self.parse_type()?)
        } else { None };
        self.expect(Token::Newline)?;
        let body = self.parse_block()?;
        Ok(Item::Function {
            sig: FnSig { name, params, ret, is_method },
            is_unsafe,
            body,
        })
    }

    fn parse_params(&mut self) -> Result<(Vec<(String, TypeAst)>, bool)> {
        let mut params = Vec::new();
        let mut is_method = false;
        if !self.matches_kw(Keyword::Takes) {
            return Ok((params, false));
        }
        self.advance();
        loop {
            if self.matches_kw(Keyword::SelfKw) {
                self.advance();
                params.push(("self".to_string(), TypeAst::Named("Self".into())));
                is_method = true;
                if self.matches(Token::Comma) { self.advance(); continue; }
                break;
            }
            let pname = self.expect_ident()?;
            self.expect_kw(Keyword::As)?;
            let ty = self.parse_type()?;
            params.push((pname, ty));
            if self.matches(Token::Comma) { self.advance(); continue; }
            break;
        }
        Ok((params, is_method))
    }

    fn parse_structure(&mut self) -> Result<Item> {
        self.expect_kw(Keyword::Structure)?;
        let name = self.expect_ident()?;
        self.expect(Token::Newline)?;
        self.expect(Token::Indent)?;
        let mut fields = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::Dedent) => { self.advance(); break; }
                Some(Token::Newline) => { self.advance(); continue; }
                Some(_) => {
                    let fname = self.expect_ident()?;
                    self.expect_kw(Keyword::As)?;
                    let ty = self.parse_type()?;
                    let default = if self.matches(Token::Eq) {
                        self.advance();
                        Some(self.parse_expr()?)
                    } else { None };
                    self.skip_newlines();
                    fields.push((fname, ty, default));
                }
                None => return Err(Error::parser(
                    "неожиданный конец в structure", self.line, self.col)),
            }
        }
        Ok(Item::Structure { name, fields })
    }

    fn parse_enumeration(&mut self) -> Result<Item> {
        self.expect_kw(Keyword::Enumeration)?;
        let name = self.expect_ident()?;
        self.expect(Token::Newline)?;
        self.expect(Token::Indent)?;
        let mut variants = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::Dedent) => { self.advance(); break; }
                Some(Token::Newline) => { self.advance(); continue; }
                Some(Token::Ident(v)) => {
                    self.advance();
                    variants.push(v);
                    self.skip_newlines();
                }
                other => return Err(Error::parser(
                    format!("ожидался вариант перечисления, получено {:?}", other),
                    self.line, self.col)),
            }
        }
        Ok(Item::Enumeration { name, variants })
    }

    fn parse_protocol(&mut self) -> Result<Item> {
        self.expect_kw(Keyword::Protocol)?;
        let name = self.expect_ident()?;
        self.expect(Token::Newline)?;
        self.expect(Token::Indent)?;
        let mut methods = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::Dedent) => { self.advance(); break; }
                Some(Token::Newline) => { self.advance(); continue; }
                Some(Token::Kw(Keyword::Function)) => {
                    self.advance();
                    let mname = self.expect_ident()?;
                    let (params, _) = self.parse_params()?;
                    let ret = if self.matches_kw(Keyword::Gives) {
                        self.advance();
                        Some(self.parse_type()?)
                    } else { None };
                    self.skip_newlines();
                    methods.push(FnSig { name: mname, params, ret, is_method: true });
                }
                other => return Err(Error::parser(
                    format!("ожидался метод в protocol, получено {:?}", other),
                    self.line, self.col)),
            }
        }
        Ok(Item::Protocol { name, methods })
    }

    fn parse_implementation(&mut self) -> Result<Item> {
        self.expect_kw(Keyword::Implementation)?;
        let for_type = self.parse_type()?;
        let protocol = if self.matches_kw(Keyword::As) {
            self.advance();
            Some(self.expect_ident()?)
        } else { None };
        self.expect(Token::Newline)?;
        self.expect(Token::Indent)?;
        let mut methods = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::Dedent) => { self.advance(); break; }
                Some(Token::Newline) => { self.advance(); continue; }
                Some(Token::Kw(Keyword::Function)) => {
                    methods.push(self.parse_function(false)?);
                }
                other => return Err(Error::parser(
                    format!("ожидался function в implementation, получено {:?}", other),
                    self.line, self.col)),
            }
        }
        Ok(Item::Implementation { for_type, protocol, methods })
    }

    fn parse_static(&mut self) -> Result<Item> {
        self.expect_kw(Keyword::Static)?;
        let mutable = self.matches_kw(Keyword::Mutable);
        if mutable { self.advance(); }
        let name = self.expect_ident()?;
        self.expect_kw(Keyword::As)?;
        let ty = self.parse_type()?;
        let init = if self.matches(Token::Eq) {
            self.advance();
            Some(self.parse_expr()?)
        } else { None };
        self.skip_newlines();
        Ok(Item::Static { name, ty, mutable, init })
    }

    fn parse_external(&mut self) -> Result<Item> {
        self.expect_kw(Keyword::External)?;
        let abi = match self.advance().cloned() {
            Some(Token::StrLit(s)) => s,
            other => return Err(Error::parser(
                format!("external требует ABI-строку, получено {:?}", other),
                self.line, self.col)),
        };
        self.expect(Token::Newline)?;
        self.expect(Token::Indent)?;
        let mut sigs = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::Dedent) => { self.advance(); break; }
                Some(Token::Newline) => { self.advance(); continue; }
                Some(Token::Kw(Keyword::Function)) => {
                    self.advance();
                    let name = self.expect_ident()?;
                    let (params, is_method) = self.parse_params()?;
                    let ret = if self.matches_kw(Keyword::Gives) {
                        self.advance();
                        Some(self.parse_type()?)
                    } else { None };
                    self.skip_newlines();
                    sigs.push(FnSig { name, params, ret, is_method });
                }
                other => return Err(Error::parser(
                    format!("ожидался function в external, получено {:?}", other),
                    self.line, self.col)),
            }
        }
        Ok(Item::External { abi, sigs })
    }

    fn parse_rule(&mut self) -> Result<Item> {
        self.expect_kw(Keyword::Rule)?;
        self.expect_kw(Keyword::When)?;
        let when_type = self.expect_ident()?;
        self.expect(Token::Dot)?;
        let when_event = self.expect_ident()?;
        let params = if self.matches_kw(Keyword::Takes) {
            self.advance();
            let mut p = Vec::new();
            loop {
                let n = self.expect_ident()?;
                self.expect_kw(Keyword::As)?;
                let t = self.parse_type()?;
                p.push((n, t));
                if self.matches(Token::Comma) { self.advance(); continue; }
                break;
            }
            p
        } else { Vec::new() };
        self.expect_kw(Keyword::Then)?;
        self.skip_newlines();
        let body = if self.matches(Token::Indent) {
            self.advance();
            self.parse_block_body()?
        } else {
            let s = self.parse_stmt()?;
            vec![s]
        };
        Ok(Item::Rule { when_type, when_event, params, body })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>> {
        self.expect(Token::Indent)?;
        self.parse_block_body()
    }

    fn parse_block_body(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::Dedent) => { self.advance(); break; }
                Some(Token::Newline) => { self.advance(); continue; }
                Some(Token::Eof) | None => break,
                Some(_) => stmts.push(self.parse_stmt()?),
            }
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        match self.peek().cloned() {
            Some(Token::Kw(Keyword::Let)) => {
                self.advance();
                let name = self.expect_ident()?;
                let ty = if self.matches_kw(Keyword::As) {
                    self.advance();
                    Some(self.parse_type()?)
                } else { None };
                self.expect(Token::Eq)?;
                let e = self.parse_expr()?;
                self.skip_newlines();
                Ok(Stmt::Let(name, ty, e))
            }
            Some(Token::Kw(Keyword::Return)) => {
                self.advance();
                if self.matches(Token::Newline)
                    || self.matches(Token::Dedent)
                    || self.matches(Token::Eof)
                {
                    self.skip_newlines();
                    Ok(Stmt::Return(None))
                } else {
                    let e = self.parse_expr()?;
                    self.skip_newlines();
                    Ok(Stmt::Return(Some(e)))
                }
            }
            Some(Token::Kw(Keyword::If)) => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(Token::Newline)?;
                let then = self.parse_block()?;
                let mut els = Vec::new();
                let save = self.pos;
                self.skip_newlines();
                if self.matches_kw(Keyword::Otherwise) {
                    self.advance();
                    self.expect(Token::Newline)?;
                    els = self.parse_block()?;
                } else {
                    self.pos = save;
                }
                Ok(Stmt::If(cond, then, els))
            }
            Some(Token::Kw(Keyword::While)) => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(Token::Newline)?;
                let body = self.parse_block()?;
                Ok(Stmt::While(cond, body))
            }
            Some(Token::Kw(Keyword::Unsafe)) => {
                self.advance();
                self.expect(Token::Newline)?;
                let body = self.parse_block()?;
                Ok(Stmt::Unsafe(body))
            }
            Some(Token::Kw(Keyword::Assembly)) => {
                self.advance();
                self.expect(Token::Newline)?;
                self.expect(Token::Indent)?;
                let mut ops = Vec::new();
                loop {
                    match self.peek().cloned() {
                        Some(Token::Dedent) => { self.advance(); break; }
                        Some(Token::Newline) => { self.advance(); continue; }
                        Some(Token::Kw(Keyword::Output)) => {
                            self.advance();
                            self.expect(Token::LParen)?;
                            let p = self.parse_expr()?;
                            self.expect(Token::Comma)?;
                            let v = self.parse_expr()?;
                            self.expect(Token::RParen)?;
                            self.skip_newlines();
                            ops.push(AsmOp::Output(p, v));
                        }
                        Some(Token::Kw(Keyword::Input)) => {
                            self.advance();
                            self.expect(Token::LParen)?;
                            let p = self.parse_expr()?;
                            self.expect(Token::RParen)?;
                            self.skip_newlines();
                            ops.push(AsmOp::Input(p));
                        }
                        Some(Token::Kw(Keyword::Interrupt)) => {
                            self.advance();
                            self.expect(Token::LParen)?;
                            let n = self.parse_expr()?;
                            self.expect(Token::RParen)?;
                            self.skip_newlines();
                            ops.push(AsmOp::Interrupt(n));
                        }
                        Some(Token::Kw(Keyword::Halt)) => {
                            self.advance();
                            self.skip_newlines();
                            ops.push(AsmOp::Halt);
                        }
                        Some(Token::Kw(Keyword::No)) => {
                            self.advance();
                            self.expect_kw(Keyword::Operation)?;
                            self.skip_newlines();
                            ops.push(AsmOp::NoOperation);
                        }
                        other => return Err(Error::parser(
                            format!("asm: неожиданное {:?}", other),
                            self.line, self.col)),
                    }
                }
                Ok(Stmt::Assembly(ops))
            }
            Some(Token::Kw(Keyword::Panic)) => {
                self.advance();
                let msg = match self.advance().cloned() {
                    Some(Token::StrLit(s)) => s,
                    other => return Err(Error::parser(
                        format!("panic требует строку, получено {:?}", other),
                        self.line, self.col)),
                };
                self.skip_newlines();
                Ok(Stmt::Panic(msg))
            }
            Some(Token::Kw(Keyword::Emit)) => {
                self.advance();
                let ev = self.expect_ident()?;
                self.skip_newlines();
                Ok(Stmt::Emit(ev))
            }
            Some(Token::Kw(Keyword::Show)) => {
                self.advance();
                // в контексте show верхнеуровневый 'as' — разделитель формата, не cast
                let saved = self.no_cast_as;
                self.no_cast_as = true;
                let e_result = self.parse_expr();
                self.no_cast_as = saved;
                let e = e_result?;
                self.expect_kw(Keyword::As)?;
                let fmt = self.expect_ident()?;
                self.skip_newlines();
                Ok(Stmt::Show(e, fmt))
            }
            Some(Token::Kw(Keyword::For)) => {
                self.advance();
                if self.matches_kw(Keyword::Each) {
                    self.advance();
                }
                let var = self.expect_ident()?;
                self.expect_kw(Keyword::In)?;
                let iter = self.parse_expr()?;
                self.expect(Token::Newline)?;
                let body = self.parse_block()?;
                Ok(Stmt::ForEach(var, iter, body))
            }
            Some(_) => {
                let e = self.parse_expr()?;
                if self.matches(Token::Eq) {
                    self.advance();
                    let v = self.parse_expr()?;
                    self.skip_newlines();
                    Ok(Stmt::Assign(e, v))
                } else {
                    self.skip_newlines();
                    Ok(Stmt::ExprStmt(e))
                }
            }
            None => Err(Error::parser("неожиданный конец", self.line, self.col)),
        }
    }

    fn parse_type(&mut self) -> Result<TypeAst> {
        if self.matches(Token::Star) {
            self.advance();
            let inner = self.parse_type()?;
            return Ok(TypeAst::Pointer(Box::new(inner)));
        }
        match self.advance().cloned() {
            Some(Token::Ty(t))     => Ok(TypeAst::Prim(t)),
            Some(Token::Ident(s))  => Ok(TypeAst::Named(s)),
            other => Err(Error::parser(
                format!("ожидался тип, получено {:?}", other),
                self.line, self.col)),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr> { self.parse_or() }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut e = self.parse_and()?;
        while self.is_ident_kw("or") {
            self.advance();
            let r = self.parse_and()?;
            e = Expr::Bin(BinOp::Or, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut e = self.parse_bit_or()?;
        while self.is_ident_kw("and") {
            self.advance();
            let r = self.parse_bit_or()?;
            e = Expr::Bin(BinOp::And, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_bit_or(&mut self) -> Result<Expr> {
        let mut e = self.parse_bit_xor()?;
        while self.matches(Token::Pipe) {
            self.advance();
            let r = self.parse_bit_xor()?;
            e = Expr::Bin(BinOp::BitOr, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_bit_xor(&mut self) -> Result<Expr> {
        let mut e = self.parse_bit_and()?;
        while self.matches(Token::Caret) {
            self.advance();
            let r = self.parse_bit_and()?;
            e = Expr::Bin(BinOp::BitXor, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_bit_and(&mut self) -> Result<Expr> {
        let mut e = self.parse_equality()?;
        while self.matches(Token::Amp) {
            self.advance();
            let r = self.parse_equality()?;
            e = Expr::Bin(BinOp::BitAnd, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut e = self.parse_cmp()?;
        loop {
            if self.matches(Token::EqEq) {
                self.advance();
                let r = self.parse_cmp()?;
                e = Expr::Bin(BinOp::Eq, Box::new(e), Box::new(r));
            } else if self.matches(Token::Ne) {
                self.advance();
                let r = self.parse_cmp()?;
                e = Expr::Bin(BinOp::Ne, Box::new(e), Box::new(r));
            } else { break; }
        }
        Ok(e)
    }

    fn parse_cmp(&mut self) -> Result<Expr> {
        let mut e = self.parse_shift()?;
        loop {
            let op = match self.peek() {
                Some(Token::Lt) => Some(BinOp::Lt),
                Some(Token::Gt) => Some(BinOp::Gt),
                Some(Token::Le) => Some(BinOp::Le),
                Some(Token::Ge) => Some(BinOp::Ge),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let r = self.parse_shift()?;
                e = Expr::Bin(op, Box::new(e), Box::new(r));
            } else { break; }
        }
        Ok(e)
    }

    fn parse_shift(&mut self) -> Result<Expr> {
        let mut e = self.parse_add()?;
        loop {
            if self.matches(Token::Shl) {
                self.advance();
                let r = self.parse_add()?;
                e = Expr::Bin(BinOp::Shl, Box::new(e), Box::new(r));
            } else if self.matches(Token::Shr) {
                self.advance();
                let r = self.parse_add()?;
                e = Expr::Bin(BinOp::Shr, Box::new(e), Box::new(r));
            } else { break; }
        }
        Ok(e)
    }

    fn parse_add(&mut self) -> Result<Expr> {
        let mut e = self.parse_mul()?;
        loop {
            if self.matches(Token::Plus) {
                self.advance();
                let r = self.parse_mul()?;
                e = Expr::Bin(BinOp::Add, Box::new(e), Box::new(r));
            } else if self.matches(Token::Minus) {
                self.advance();
                let r = self.parse_mul()?;
                e = Expr::Bin(BinOp::Sub, Box::new(e), Box::new(r));
            } else { break; }
        }
        Ok(e)
    }

    fn parse_mul(&mut self) -> Result<Expr> {
        let mut e = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star)    => Some(BinOp::Mul),
                Some(Token::Slash)   => Some(BinOp::Div),
                Some(Token::Percent) => Some(BinOp::Mod),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let r = self.parse_unary()?;
                e = Expr::Bin(op, Box::new(e), Box::new(r));
            } else { break; }
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if self.matches(Token::Minus) {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Un(UnOp::Neg, Box::new(e)));
        }
        if self.matches(Token::Tilde) {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Un(UnOp::BitNot, Box::new(e)));
        }
        if self.is_ident_kw("not") {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Un(UnOp::Not, Box::new(e)));
        }
        if self.matches(Token::Star) {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Deref(Box::new(e)));
        }
        if self.matches(Token::Amp) {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::AddrOf(Box::new(e)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if self.matches(Token::Dot) {
                self.advance();
                let name = self.expect_ident()?;
                if self.matches(Token::LParen) {
                    self.advance();
                    let args = self.parse_args()?;
                    e = Expr::MethodCall(Box::new(e), name, args);
                } else {
                    e = Expr::Field(Box::new(e), name);
                }
            } else if !self.no_cast_as && self.matches(Token::Kw(Keyword::As)) {
                self.advance();
                let ty = self.parse_type()?;
                e = Expr::Cast(Box::new(e), ty);
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        if self.matches(Token::RParen) {
            self.advance();
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr()?);
            if self.matches(Token::Comma) { self.advance(); continue; }
            break;
        }
        self.expect(Token::RParen)?;
        Ok(args)
    }

        fn parse_primary(&mut self) -> Result<Expr> {
        match self.advance().cloned() {
            Some(Token::IntLit(n))   => Ok(Expr::Int(n)),
            Some(Token::FloatLit(f)) => Ok(Expr::Float(f)),
            Some(Token::StrLit(s))   => Ok(Expr::Str(s)),
            Some(Token::BoolLit(b))  => Ok(Expr::Bool(b)),
            Some(Token::Kw(Keyword::SelfKw)) => Ok(Expr::Var("self".into())),
            Some(Token::Kw(Keyword::Assembly)) => {
                // assembly как выражение: assembly input (port)
                self.expect_kw(Keyword::Input)?;
                self.expect(Token::LParen)?;
                let saved = self.no_cast_as;
                self.no_cast_as = false;
                let port = self.parse_expr()?;
                self.no_cast_as = saved;
                self.expect(Token::RParen)?;
                Ok(Expr::AsmInput(Box::new(port)))
            }
            Some(Token::Ident(name)) => {
                if self.matches(Token::LParen) {
                    self.advance();
                    let args = self.parse_args()?;
                    Ok(Expr::Call(name, args))
                } else if self.matches(Token::LBrace) {
                    self.advance();
                    let saved = self.no_cast_as;
                    self.no_cast_as = false;
                    let mut fields = Vec::new();
                    loop {
                        if self.matches(Token::RBrace) { self.advance(); break; }
                        let f = self.expect_ident()?;
                        self.expect(Token::Eq)?;
                        let v = self.parse_expr()?;
                        fields.push((f, v));
                        if self.matches(Token::Comma) { self.advance(); continue; }
                    }
                    self.no_cast_as = saved;
                    Ok(Expr::StructLit(name, fields))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Some(Token::LParen) => {
                let saved = self.no_cast_as;
                self.no_cast_as = false;
                let e_result = self.parse_expr();
                self.no_cast_as = saved;
                let e = e_result?;
                self.expect(Token::RParen)?;
                Ok(e)
            }
            other => Err(Error::parser(
                format!("ожидалось выражение, получено {:?}", other),
                self.line, self.col)),
        }
    }

    fn peek(&self) -> Option<&Token> { self.toks.get(self.pos) }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.toks.get(self.pos);
        if t.is_some() { self.pos += 1; }
        t
    }

    fn matches(&self, t: Token) -> bool {
        self.peek() == Some(&t)
    }

    fn matches_kw(&self, k: Keyword) -> bool {
        matches!(self.peek(), Some(Token::Kw(ref kk)) if *kk == k)
    }

    fn is_ident_kw(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Token::Ident(x)) if x == s)
    }

    fn expect(&mut self, t: Token) -> Result<()> {
        if self.matches(t.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(Error::parser(
                format!("ожидалось {:?}, получено {:?}", t, self.peek()),
                self.line, self.col))
        }
    }

    fn expect_kw(&mut self, k: Keyword) -> Result<()> {
        if self.matches_kw(k.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(Error::parser(
                format!("ожидалось {:?}, получено {:?}", k, self.peek()),
                self.line, self.col))
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.advance().cloned() {
            Some(Token::Ident(s)) => Ok(s),
            other => Err(Error::parser(
                format!("ожидался идентификатор, получено {:?}", other),
                self.line, self.col)),
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Token::Newline)) {
            self.advance();
        }
    }
}