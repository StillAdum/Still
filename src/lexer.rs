use crate::span::{err, Diag, Span};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum StrPart {
    Lit(String),
    Code(String, Span),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kw {
    Func,
    Type,
    Extern,
    Use,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    And,
    Or,
    Not,
    Is,
    Break,
    Continue,
    Return,
    True,
    False,
    None,
    Pass,
    As,
}

#[derive(Clone, Debug, PartialEq)]
pub enum P {
    LP,
    RP,
    LB,
    RB,
    LC,
    RC,
    Comma,
    Colon,
    Dot,
    Arrow,
    Plus,
    Minus,
    Star,
    Slash,
    DSlash,
    Percent,
    DStar,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    DSlashEq,
    DStarEq,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Question,
    At,
}

#[derive(Clone, Debug)]
pub enum Tok {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(Vec<StrPart>),
    Kw(Kw),
    P(P),
    Newline,
    Indent,
    Dedent,
    Err(String),
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

fn keyword(s: &str) -> Option<Kw> {
    Some(match s {
        "func" => Kw::Func,
        "type" => Kw::Type,
        "extern" => Kw::Extern,
        "use" => Kw::Use,
        "if" => Kw::If,
        "elif" => Kw::Elif,
        "else" => Kw::Else,
        "while" => Kw::While,
        "for" => Kw::For,
        "in" => Kw::In,
        "and" => Kw::And,
        "or" => Kw::Or,
        "not" => Kw::Not,
        "is" => Kw::Is,
        "break" => Kw::Break,
        "continue" => Kw::Continue,
        "return" => Kw::Return,
        "true" => Kw::True,
        "false" => Kw::False,
        "none" => Kw::None,
        "pass" => Kw::Pass,
        "as" => Kw::As,
        _ => return None,
    })
}

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    file: Rc<str>,
    indents: Vec<usize>,
    depth: i32,
    pending_newline: bool,
    pub toks: Vec<Token>,
    pub diags: Vec<Diag>,
}

impl Lexer {
    pub fn new(src: &str, file: &str) -> Lexer {
        Lexer {
            src: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            file: Rc::from(file),
            indents: vec![0],
            depth: 0,
            pending_newline: false,
            toks: Vec::new(),
            diags: Vec::new(),
        }
    }

    fn span(&self) -> Span {
        Span::new(self.file.clone(), self.line, self.col)
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.src.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn push(&mut self, tok: Tok, span: Span) {
        self.toks.push(Token { tok, span });
    }

    pub fn run(mut self) -> (Vec<Token>, Vec<Diag>) {
        self.handle_indent();
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => break,
            };
            match c {
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '#' => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                '\\' if matches!(self.peek2(), Some('\n')) => {
                    self.bump();
                    self.bump();
                }
                ';' => {
                    self.bump();
                    if self.depth == 0 {
                        if let Some(last) = self.toks.last() {
                            if !matches!(last.tok, Tok::Newline | Tok::Indent | Tok::Dedent | Tok::Eof) {
                                let sp = self.span();
                                self.push(Tok::Newline, sp);
                            }
                        }
                    }
                }
                '\n' => {
                    self.bump();
                    if self.depth > 0 {
                        continue;
                    }
                    self.pending_newline = true;
                    self.handle_indent();
                }
                '0'..='9' => self.lex_number(),
                '"' | '\'' => self.lex_string(),
                c if c.is_ascii_alphabetic() || c == '_' || (c as u32) >= 0x80 => self.lex_ident(),
                _ => self.lex_punct(),
            }
        }
        let sp = self.span();
        self.flush_newline();
        while self.indents.len() > 1 {
            self.indents.pop();
            self.push(Tok::Dedent, sp.clone());
        }
        self.push(Tok::Eof, sp);
        (self.toks, self.diags)
    }

    fn skip_blank(&mut self) {
        loop {
            let mut i = self.pos;
            let mut w = 0usize;
            while let Some(&c) = self.src.get(i) {
                match c {
                    ' ' => {
                        w += 1;
                        i += 1;
                    }
                    '\t' => {
                        w += 4;
                        i += 1;
                    }
                    '\r' => {
                        i += 1;
                    }
                    _ => break,
                }
            }
            match self.src.get(i) {
                Some('\n') => {
                    for _ in 0..(i - self.pos) {
                        self.bump();
                    }
                    self.bump();
                }
                Some('#') => {
                    for _ in 0..(i - self.pos) {
                        self.bump();
                    }
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => {
                    for _ in 0..(i - self.pos) {
                        self.bump();
                    }
                    self.col = 1 + w as u32;
                    return;
                }
            }
        }
    }

    fn handle_indent(&mut self) {
        if self.pos >= self.src.len() {
            return;
        }
        self.skip_blank();
        if self.pos >= self.src.len() {
            return;
        }
        self.flush_newline();
        let w = (self.col - 1) as usize;
        let top = *self.indents.last().unwrap();
        let sp = self.span();
        if w > top {
            self.indents.push(w);
            self.push(Tok::Indent, sp);
        } else if w < top {
            while *self.indents.last().unwrap() > w {
                self.indents.pop();
                self.push(Tok::Dedent, sp.clone());
            }
            if *self.indents.last().unwrap() != w {
                self.diags.push(err(
                    &sp,
                    format!("inconsistent indentation: this line dedents to {} spaces but no open block starts there", w),
                ));
            }
        }
    }

    fn flush_newline(&mut self) {
        if self.pending_newline {
            self.pending_newline = false;
            if let Some(last) = self.toks.last() {
                if !matches!(last.tok, Tok::Newline | Tok::Indent | Tok::Dedent | Tok::Eof) {
                    let sp = self.span();
                    self.push(Tok::Newline, sp);
                }
            }
        }
    }

    fn lex_number(&mut self) {
        let sp = self.span();
        let start = self.pos;
        if self.peek() == Some('0')
            && matches!(self.peek2(), Some('x') | Some('X') | Some('o') | Some('O') | Some('b') | Some('B'))
        {
            self.bump();
            let base_c = self.bump().unwrap();
            let base = match base_c {
                'x' | 'X' => 16,
                'o' | 'O' => 8,
                _ => 2,
            };
            let mut digits = String::new();
            while let Some(c) = self.peek() {
                if c == '_' {
                    self.bump();
                } else if c.is_digit(base) {
                    digits.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            if digits.is_empty() {
                self.diags.push(err(&sp, format!("missing digits after '0{}'", base_c)));
                self.push(Tok::Err("bad number".into()), sp);
                return;
            }
            match i64::from_str_radix(&digits, base) {
                Ok(v) => self.push(Tok::Int(v), sp),
                Err(_) => {
                    self.diags.push(err(&sp, "integer literal too large (max is 9223372036854775807)".into()));
                    self.push(Tok::Err("int overflow".into()), sp);
                }
            }
            return;
        }
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') {
            if let Some(c2) = self.peek2() {
                if c2.is_ascii_digit() {
                    is_float = true;
                    self.bump();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() || c == '_' {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            let mut look = self.pos + 1;
            if matches!(self.src.get(look), Some('+') | Some('-')) {
                look += 1;
            }
            if matches!(self.src.get(look), Some(c) if c.is_ascii_digit()) {
                is_float = true;
                self.bump();
                if matches!(self.peek(), Some('+') | Some('-')) {
                    self.bump();
                }
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() || c == '_' {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
        }
        let text: String = self.src[start..self.pos].iter().filter(|c| **c != '_').collect();
        if is_float {
            match text.parse::<f64>() {
                Ok(v) => self.push(Tok::Float(v), sp),
                Err(_) => {
                    self.diags.push(err(&sp, "bad float literal".into()));
                    self.push(Tok::Err("bad float".into()), sp);
                }
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => self.push(Tok::Int(v), sp),
                Err(_) => {
                    self.diags.push(err(&sp, "integer literal too large (max is 9223372036854775807)".into()));
                    self.push(Tok::Err("int overflow".into()), sp);
                }
            }
        }
    }

    fn lex_string(&mut self) {
        let sp = self.span();
        let quote = self.bump().unwrap();
        let mut parts: Vec<StrPart> = Vec::new();
        let mut lit = String::new();
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => {
                    self.diags.push(err(&sp, "unterminated string".into()));
                    self.push(Tok::Err("unterminated string".into()), sp);
                    return;
                }
            };
            if c == '\n' {
                self.diags.push(err(&sp, "unterminated string (strings cannot span lines; join with +)".into()));
                self.push(Tok::Err("unterminated string".into()), sp);
                return;
            }
            if c == quote {
                self.bump();
                break;
            }
            if c == '\\' {
                self.bump();
                let e = match self.bump() {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('0') => '\0',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    Some('"') => '"',
                    Some('{') => {
                        lit.push('{');
                        continue;
                    }
                    Some(other) => {
                        let esp = self.span();
                        self.diags.push(err(&esp, format!("unknown escape '\\{}' in string", other)));
                        other
                    }
                    None => {
                        self.diags.push(err(&sp, "unterminated string".into()));
                        self.push(Tok::Err("unterminated string".into()), sp);
                        return;
                    }
                };
                lit.push(e);
                continue;
            }
            if c == '{' {
                if self.peek2() == Some('{') {
                    self.bump();
                    self.bump();
                    lit.push('{');
                    continue;
                }
                self.bump();
                let code_sp = self.span();
                let mut code = String::new();
                let mut depth = 1usize;
                let mut closed = false;
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    if c == '{' {
                        depth += 1;
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            self.bump();
                            closed = true;
                            break;
                        }
                    }
                    code.push(c);
                    self.bump();
                }
                if !closed {
                    self.diags.push(err(&code_sp, "unterminated '{' interpolation in string".into()));
                    self.push(Tok::Err("bad interpolation".into()), sp);
                    return;
                }
                if code.trim().is_empty() {
                    self.diags.push(err(&code_sp, "empty '{}' interpolation in string".into()));
                }
                if !lit.is_empty() {
                    parts.push(StrPart::Lit(std::mem::take(&mut lit)));
                }
                parts.push(StrPart::Code(code, code_sp));
                continue;
            }
            if c == '}' && self.peek2() == Some('}') {
                self.bump();
                self.bump();
                lit.push('}');
                continue;
            }
            lit.push(c);
            self.bump();
        }
        if !lit.is_empty() || parts.is_empty() {
            parts.push(StrPart::Lit(lit));
        }
        self.push(Tok::Str(parts), sp);
    }

    fn lex_ident(&mut self) {
        let sp = self.span();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || (c as u32) >= 0x80 {
                self.bump();
            } else {
                break;
            }
        }
        let name: String = self.src[start..self.pos].iter().collect();
        match keyword(&name) {
            Some(k) => self.push(Tok::Kw(k), sp),
            None => self.push(Tok::Ident(name), sp),
        }
    }

    fn lex_punct(&mut self) {
        let sp = self.span();
        let c = self.bump().unwrap();
        let two = |l: &Lexer| l.peek();
        let tok = match c {
            '(' => {
                self.depth += 1;
                P::LP
            }
            ')' => {
                self.depth -= 1;
                P::RP
            }
            '[' => {
                self.depth += 1;
                P::LB
            }
            ']' => {
                self.depth -= 1;
                P::RB
            }
            '{' => {
                self.depth += 1;
                P::LC
            }
            '}' => {
                self.depth -= 1;
                P::RC
            }
            ',' => P::Comma,
            ':' => P::Colon,
            '.' => P::Dot,
            '+' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::PlusEq
                }
                _ => P::Plus,
            },
            '-' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::MinusEq
                }
                Some('>') => {
                    self.bump();
                    P::Arrow
                }
                _ => P::Minus,
            },
            '*' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::StarEq
                }
                Some('*') => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        P::DStarEq
                    } else {
                        P::DStar
                    }
                }
                _ => P::Star,
            },
            '/' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::SlashEq
                }
                Some('/') => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        P::DSlashEq
                    } else {
                        P::DSlash
                    }
                }
                _ => P::Slash,
            },
            '%' => P::Percent,
            '=' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::EqEq
                }
                _ => P::Eq,
            },
            '!' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::NotEq
                }
                _ => {
                    self.diags.push(err(&sp, "unexpected '!' (use not / !=)".into()));
                    P::NotEq
                }
            },
            '<' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::Le
                }
                _ => P::Lt,
            },
            '>' => match two(self) {
                Some('=') => {
                    self.bump();
                    P::Ge
                }
                _ => P::Gt,
            },
            '?' => P::Question,
            '@' => P::At,
            other => {
                self.diags.push(err(&sp, format!("unexpected character '{}'", other)));
                P::Dot
            }
        };
        self.push(Tok::P(tok), sp);
    }
}
