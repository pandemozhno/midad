use crate::error::{Error, Result};
use crate::profile::Profile;
use crate::token::{Keyword, Token};

pub struct Lexer<'a> {
    src: &'a [char],
    pos: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    profile: &'a Profile,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, profile: &'a Profile) -> Self {
        let chars: Vec<char> = src.chars().collect();
        Self {
            src: Box::leak(chars.into_boxed_slice()),
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            profile,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut out = Vec::new();
        let mut at_line_start = true;

        loop {
            if at_line_start {
                if let Some(err) = self.measure_indent(&mut out)? {
                    return Err(err);
                }
                at_line_start = false;
            }

            if let Some(c) = self.peek() {
                if c == ' ' || c == '\t' {
                    self.advance();
                    continue;
                }
                if c == '\n' {
                    self.advance();
                    self.line += 1;
                    self.col = 1;
                    out.push(Token::Newline);
                    at_line_start = true;
                    continue;
                }
                if c == '-' && self.peek_at(1) == Some('-') {
                    while let Some(c) = self.peek() {
                        if c == '\n' { break; }
                        self.advance();
                    }
                    continue;
                }
            }

            if self.peek().is_none() { break; }

            if let Some(tok) = self.next_token()? {
                out.push(tok);
            }
        }

        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            out.push(Token::Dedent);
        }
        out.push(Token::Eof);
        Ok(out)
    }

    fn measure_indent(&mut self, out: &mut Vec<Token>) -> Result<Option<Error>> {
        let mut tabs = 0;
        while self.peek() == Some('\t') {
            self.advance();
            tabs += 1;
        }
        if self.peek() == Some(' ') {
            return Ok(Some(Error::lexer(
                "пробелы в начале строки запрещены; только табы",
                self.line, self.col)));
        }
        if self.peek() == Some('\n') {
            return Ok(None);
        }
        if self.peek().is_none() {
            return Ok(None);
        }

        let top = *self.indent_stack.last().unwrap();
        if tabs > top {
            self.indent_stack.push(tabs);
            out.push(Token::Indent);
        } else if tabs < top {
            while self.indent_stack.len() > 1
                && *self.indent_stack.last().unwrap() > tabs
            {
                self.indent_stack.pop();
                out.push(Token::Dedent);
            }
            if *self.indent_stack.last().unwrap() != tabs {
                return Ok(Some(Error::lexer(
                    "непостоянные отступы", self.line, self.col)));
            }
        }
        Ok(None)
    }

    fn next_token(&mut self) -> Result<Option<Token>> {
        let c = match self.peek() { Some(c) => c, None => return Ok(None) };

        if c.is_ascii_digit() {
            return Ok(Some(self.lex_number()?));
        }
        if c == '"' {
            return Ok(Some(self.lex_string()?));
        }
        if c == '@' {
            return Ok(Some(self.lex_at_ident()));
        }
        if c.is_alphabetic() || c == '_' {
            return Ok(Some(self.lex_ident()));
        }

        if let Some(tok) = self.two_char_op() {
            return Ok(Some(tok));
        }
        if let Some(tok) = self.one_char_op() {
            return Ok(Some(tok));
        }

        Err(Error::lexer(format!("неизвестный символ '{}'", c), self.line, self.col))
    }

    fn lex_at_ident(&mut self) -> Token {
        let mut s = String::new();
        s.push('@');
        self.advance();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Token::Ident(s)
    }

    fn lex_number(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_col = self.col;
        let mut digits = String::new();
        let mut radix = 10;

        if self.peek() == Some('0') {
            match self.peek_at(1) {
                Some('x') | Some('X') => { self.advance(); self.advance(); radix = 16; }
                Some('b') | Some('B') => { self.advance(); self.advance(); radix = 2; }
                _ => {}
            }
        }

        while let Some(c) = self.peek() {
            if c == '_' { self.advance(); continue; }
            let ok = match radix {
                16 => c.is_ascii_hexdigit(),
                10 => c.is_ascii_digit(),
                2  => c == '0' || c == '1',
                _  => false,
            };
            if ok { digits.push(c); self.advance(); } else { break; }
        }

        if digits.is_empty() {
            return Err(Error::lexer("пустой числовой литерал", start_line, start_col));
        }

        if radix == 10
            && self.peek() == Some('.')
            && self.peek_at(1).map_or(false, |c| c.is_ascii_digit())
        {
            self.advance();
            digits.push('.');
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() || c == '_' {
                    digits.push(c);
                    self.advance();
                } else { break; }
            }
            let f: f64 = digits.replace('_', "").parse()
                .map_err(|_| Error::lexer("плохое дробное число", start_line, start_col))?;
            return Ok(Token::FloatLit(f));
        }

        let n = u64::from_str_radix(&digits, radix)
            .map_err(|_| Error::lexer(format!("плохое число '{}'", digits), start_line, start_col))?;
        Ok(Token::IntLit(n))
    }

    fn lex_string(&mut self) -> Result<Token> {
        let start = self.line;
        let startc = self.col;
        self.advance();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(Error::lexer("незакрытая строка", start, startc)),
                Some('"') => { self.advance(); break; }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('n')  => { s.push('\n'); self.advance(); }
                        Some('t')  => { s.push('\t'); self.advance(); }
                        Some('\\') => { s.push('\\'); self.advance(); }
                        Some('"')  => { s.push('"');  self.advance(); }
                        _ => { s.push('\\'); }
                    }
                }
                Some(c) => { s.push(c); self.advance(); }
            }
        }
        Ok(Token::StrLit(s))
    }

    fn lex_ident(&mut self) -> Token {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else { break; }
        }
        if let Some(kw) = self.profile.keywords.get(&s) {
            match kw {
                Keyword::True  => return Token::BoolLit(true),
                Keyword::False => return Token::BoolLit(false),
                k => return Token::Kw(k.clone()),
            }
        }
        if let Some(t) = self.profile.types.get(&s) {
            return Token::Ty(*t);
        }
        // локализованное имя → каноническое (language independence в SEM)
        if let Some(canonical) = self.profile.names.get(&s) {
            return Token::Ident(canonical.clone());
        }
        Token::Ident(s)
    }

    fn two_char_op(&mut self) -> Option<Token> {
        let a = self.peek()?;
        let b = self.peek_at(1)?;
        let tok = match (a, b) {
            ('=', '=') => Token::EqEq,
            ('!', '=') => Token::Ne,
            ('<', '=') => Token::Le,
            ('>', '=') => Token::Ge,
            ('<', '<') => Token::Shl,
            ('>', '>') => Token::Shr,
            ('-', '>') => Token::Arrow,
            _ => return None,
        };
        self.advance(); self.advance();
        Some(tok)
    }

    fn one_char_op(&mut self) -> Option<Token> {
        let c = self.peek()?;
        let tok = match c {
            '=' => Token::Eq,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '&' => Token::Amp,
            '|' => Token::Pipe,
            '^' => Token::Caret,
            '~' => Token::Tilde,
            ':' => Token::Colon,
            ',' => Token::Comma,
            '.' => Token::Dot,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            _ => return None,
        };
        self.advance();
        Some(tok)
    }

    fn peek(&self) -> Option<char> { self.src.get(self.pos).copied() }
    fn peek_at(&self, n: usize) -> Option<char> { self.src.get(self.pos + n).copied() }
    fn advance(&mut self) -> Option<char> {
        let c = self.src.get(self.pos).copied();
        if c.is_some() { self.pos += 1; self.col += 1; }
        c
    }
}