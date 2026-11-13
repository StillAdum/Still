use crate::ast::*;
use crate::lexer::{Lexer, StrPart, Tok, Token};
use crate::span::{err, Diag, Span};
use std::rc::Rc;

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    pub diags: Vec<Diag>,
    file: Rc<str>,
    depth: u32,
    expr_depth: u32,
}

pub fn parse(source: &str, file: &str) -> (Vec<Stmt>, Vec<Diag>) {
    let (toks, diags) = Lexer::new(source, file).run();
    let mut p = Parser {
        toks,
        pos: 0,
        diags,
        file: Rc::from(file),
        depth: 0,
        expr_depth: 0,
    };
    let stmts = p.parse_program();
    (stmts, p.diags)
}

pub fn parse_expr_src(source: &str, file: &str) -> (Option<Expr>, Vec<Diag>) {
    let (toks, diags) = Lexer::new(source, file).run();
    let mut p = Parser {
        toks,
        pos: 0,
        diags,
        file: Rc::from(file),
        depth: 0,
        expr_depth: 0,
    };
    let r = p.parse_ternary();
    if !matches!(p.cur().tok, Tok::Eof) {
        p.error(&p.cur().span, "unexpected trailing tokens after expression".into());
        return (None, p.diags);
    }
    (r, p.diags)
}

impl Parser {
    fn cur(&self) -> Token {
        self.toks[self.pos.min(self.toks.len() - 1)].clone()
    }

    fn peek(&self) -> Token {
        self.toks[(self.pos + 1).min(self.toks.len() - 1)].clone()
    }

    fn at_eof(&self) -> bool {
        matches!(self.cur().tok, Tok::Eof)
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos.min(self.toks.len() - 1)].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn error(&mut self, span: &Span, msg: String) {
        self.diags.push(err(span, msg));
    }

    fn sync_stmt(&mut self) {
        loop {
            match &self.cur().tok {
                Tok::Newline => {
                    self.bump();
                    return;
                }
                Tok::Dedent | Tok::Eof => return,
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.cur().tok, Tok::Newline) {
            self.bump();
        }
    }

    fn parse_program(&mut self) -> Vec<Stmt> {
        let mut out = Vec::new();
        self.skip_newlines();
        while !self.at_eof() {
            let before = self.pos;
            match self.parse_top_stmt() {
                Some(s) => out.push(s),
                None => self.sync_stmt(),
            }
            self.skip_newlines();
            while matches!(self.cur().tok, Tok::Dedent) {
                self.bump();
            }
            if self.pos == before {
                self.bump();
            }
        }
        out
    }

    fn parse_top_stmt(&mut self) -> Option<Stmt> {
        let mut attrs: Vec<String> = Vec::new();
        while matches!(self.cur().tok, Tok::P(crate::lexer::P::At)) {
            self.bump();
            match self.cur().tok.clone() {
                Tok::Ident(name) => {
                    attrs.push(name);
                    self.bump();
                }
                _ => {
                    let t = self.cur();
                    self.error(&t.span, "expected attribute name after @".into());
                    return None;
                }
            }
            match self.cur().tok {
                Tok::Newline => {
                    self.bump();
                }
                _ => {}
            }
        }
        match self.parse_stmt(true, attrs) {
            Some(s) => Some(s),
            None => None,
        }
    }

    fn parse_stmt(&mut self, top: bool, attrs: Vec<String>) -> Option<Stmt> {
        let sp = self.cur().span.clone();
        match self.cur().tok.clone() {
            Tok::Kw(crate::lexer::Kw::Func) => {
                if top {
                    self.parse_func(attrs, sp).map(Stmt::FuncDef)
                } else {
                    self.error(&sp, "functions can only be defined at the top level".into());
                    None
                }
            }
            Tok::Kw(crate::lexer::Kw::Type) => {
                if top {
                    self.parse_type(attrs, sp).map(Stmt::TypeDef)
                } else {
                    self.error(&sp, "types can only be defined at the top level".into());
                    None
                }
            }
            Tok::Kw(crate::lexer::Kw::Extern) => {
                if top {
                    self.parse_extern(sp).map(Stmt::Extern)
                } else {
                    self.error(&sp, "extern blocks can only appear at the top level".into());
                    None
                }
            }
            Tok::Kw(crate::lexer::Kw::Use) => {
                if top {
                    self.parse_use(sp.clone()).map(|t| Stmt::Use { target: t.0, alias: t.1, names: t.2, span: sp })
                } else {
                    self.error(&sp, "use can only appear at the top level".into());
                    None
                }
            }
            Tok::Kw(crate::lexer::Kw::If) => self.parse_if(sp).map(|s| s.0),
            Tok::Kw(crate::lexer::Kw::While) => self.parse_while(sp),
            Tok::Kw(crate::lexer::Kw::For) => self.parse_for(sp),
            // `match` is a soft keyword: only a match statement when followed by a
            // subject variable, so identifiers named `match` keep working.
            Tok::Ident(ref n) if n == "match" && matches!(self.peek().tok, Tok::Ident(_)) => {
                self.parse_match(sp)
            }
            _ => self.parse_simple_stmt(),
        }
    }

    fn parse_func(&mut self, mut attrs: Vec<String>, sp: Span) -> Option<FuncDef> {
        self.bump();
        let name = match self.cur().tok.clone() {
            Tok::Ident(n) => {
                self.bump();
                n
            }
            other => {
                self.error(&self.cur().span, format!("expected function name, found {}", tok_str(&other)));
                return None;
            }
        };
        if !matches!(self.cur().tok, Tok::P(crate::lexer::P::LP)) {
            self.error(&self.cur().span, format!("expected '(' after function name '{}'", name));
            return None;
        }
        let (params, variadic) = self.parse_params()?;
        if variadic {
            self.error(&sp, "'...' is only allowed in extern blocks".into());
        }
        let ret = if matches!(self.cur().tok, Tok::P(crate::lexer::P::Arrow)) {
            self.bump();
            Some(self.parse_annot()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        attrs.retain(|a| a != "soa");
        Some(FuncDef {
            attrs,
            name,
            params,
            ret,
            body,
            span: sp,
        })
    }

    fn parse_params(&mut self) -> Option<(Vec<Param>, bool)> {
        self.bump();
        let mut params = Vec::new();
        let mut variadic = false;
        loop {
            match self.cur().tok.clone() {
                Tok::P(crate::lexer::P::RP) => {
                    self.bump();
                    break;
                }
                Tok::P(crate::lexer::P::Dot) => {
                    let d = self.cur().span.clone();
                    self.bump();
                    if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Dot)) {
                        self.error(&d, "unexpected '.'".into());
                        return None;
                    }
                    self.bump();
                    if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Dot)) {
                        self.error(&d, "unexpected '..'".into());
                        return None;
                    }
                    self.bump();
                    if !matches!(self.cur().tok, Tok::P(crate::lexer::P::RP)) {
                        self.error(&self.cur().span, "'...' must be the last parameter".into());
                        return None;
                    }
                    self.bump();
                    variadic = true;
                    break;
                }
                Tok::Ident(name) => {
                    let psp = self.cur().span.clone();
                    self.bump();
                    let annot = if matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                        self.bump();
                        Some(self.parse_annot()?)
                    } else {
                        None
                    };
                    let default = if matches!(self.cur().tok, Tok::P(crate::lexer::P::Eq)) {
                        self.bump();
                        Some(self.parse_ternary()?)
                    } else {
                        None
                    };
                    params.push(Param {
                        name,
                        annot,
                        default,
                        span: psp,
                    });
                    match self.cur().tok {
                        Tok::P(crate::lexer::P::Comma) => {
                            self.bump();
                        }
                        Tok::P(crate::lexer::P::RP) => {}
                        _ => {
                            self.error(&self.cur().span, "expected ',' or ')' in parameter list".into());
                            return None;
                        }
                    }
                }
                other => {
                    self.error(&self.cur().span, format!("expected parameter name, found {}", tok_str(&other)));
                    return None;
                }
            }
        }
        Some((params, variadic))
    }

    fn parse_type(&mut self, attrs: Vec<String>, sp: Span) -> Option<TypeDef> {
        self.bump();
        let name = match self.cur().tok.clone() {
            Tok::Ident(n) => {
                self.bump();
                n
            }
            other => {
                self.error(&self.cur().span, format!("expected type name, found {}", tok_str(&other)));
                return None;
            }
        };
        if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
            self.error(&self.cur().span, format!("expected ':' after type name '{}'", name));
            return None;
        }
        self.bump();
        if !matches!(self.cur().tok, Tok::Newline) {
            self.error(&self.cur().span, "expected a newline and an indented body for the type".into());
            return None;
        }
        self.bump();
        if !matches!(self.cur().tok, Tok::Indent) {
            self.error(&self.cur().span, format!("expected an indented body for type '{}'", name));
            return None;
        }
        self.bump();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        loop {
            match self.cur().tok.clone() {
                Tok::Dedent => {
                    self.bump();
                    break;
                }
                Tok::Eof => break,
                Tok::Newline => {
                    self.bump();
                }
                Tok::Kw(crate::lexer::Kw::Func) => {
                    let fsp = self.cur().span.clone();
                    match self.parse_func(Vec::new(), fsp) {
                        Some(f) => methods.push(f),
                        None => {
                            self.sync_stmt();
                        }
                    }
                }
                Tok::Ident(fname) => {
                    let fsp = self.cur().span.clone();
                    self.bump();
                    if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                        self.error(&self.cur().span, format!("expected ':' after field '{}'", fname));
                        return None;
                    }
                    self.bump();
                    let annot = self.parse_annot()?;
                    let default = if matches!(self.cur().tok, Tok::P(crate::lexer::P::Eq)) {
                        self.bump();
                        Some(self.parse_ternary()?)
                    } else {
                        None
                    };
                    if !matches!(self.cur().tok, Tok::Newline) {
                        self.error(&self.cur().span, "expected a newline after field".into());
                        return None;
                    }
                    self.bump();
                    fields.push(Field {
                        name: fname,
                        annot,
                        default,
                        span: fsp,
                    });
                }
                other => {
                    self.error(&self.cur().span, format!("expected a field or 'func', found {}", tok_str(&other)));
                    return None;
                }
            }
        }
        Some(TypeDef {
            attrs,
            name,
            fields,
            methods,
            span: sp,
        })
    }

    fn parse_extern(&mut self, sp: Span) -> Option<Vec<ExternFn>> {
        self.bump();
        let conv = match self.cur().tok.clone() {
            Tok::Str(parts) => {
                self.bump();
                match parts.as_slice() {
                    [StrPart::Lit(s)] if s == "C" => "C".to_string(),
                    _ => {
                        self.error(&sp, "only extern \"C\" is supported".into());
                        return None;
                    }
                }
            }
            _ => {
                self.error(&sp, "expected \"C\" after extern".into());
                return None;
            }
        };
        let _ = conv;
        if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
            self.error(&self.cur().span, "expected ':' after extern \"C\"".into());
            return None;
        }
        self.bump();
        if !matches!(self.cur().tok, Tok::Newline) {
            self.error(&self.cur().span, "expected a newline and an indented block of function signatures".into());
            return None;
        }
        self.bump();
        if !matches!(self.cur().tok, Tok::Indent) {
            self.error(&self.cur().span, "expected an indented block of function signatures".into());
            return None;
        }
        self.bump();
        let mut fns = Vec::new();
        loop {
            match self.cur().tok.clone() {
                Tok::Dedent => {
                    self.bump();
                    break;
                }
                Tok::Eof => break,
                Tok::Newline => {
                    self.bump();
                }
                Tok::Kw(crate::lexer::Kw::Func) => {
                    let fsp = self.cur().span.clone();
                    self.bump();
                    let name = match self.cur().tok.clone() {
                        Tok::Ident(n) => {
                            self.bump();
                            n
                        }
                        other => {
                            self.error(&self.cur().span, format!("expected function name, found {}", tok_str(&other)));
                            return None;
                        }
                    };
                    if !matches!(self.cur().tok, Tok::P(crate::lexer::P::LP)) {
                        self.error(&self.cur().span, format!("expected '(' after '{}'", name));
                        return None;
                    }
                    self.bump();
                    let mut params: Vec<(String, TypeAnnot)> = Vec::new();
                    let mut variadic = false;
                    loop {
                        match self.cur().tok.clone() {
                            Tok::P(crate::lexer::P::RP) => {
                                self.bump();
                                break;
                            }
                            Tok::P(crate::lexer::P::Dot) => {
                                self.bump();
                                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Dot)) {
                                    self.error(&self.cur().span, "unexpected '.'".into());
                                    return None;
                                }
                                self.bump();
                                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Dot)) {
                                    self.error(&self.cur().span, "unexpected '..'".into());
                                    return None;
                                }
                                self.bump();
                                variadic = true;
                                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::RP)) {
                                    self.error(&self.cur().span, "'...' must be the last parameter".into());
                                    return None;
                                }
                                self.bump();
                                break;
                            }
                            Tok::Ident(pn) => {
                                self.bump();
                                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                                    self.error(&self.cur().span, format!("extern parameter '{}' needs a type (e.g. {}: int)", pn, pn));
                                    return None;
                                }
                                self.bump();
                                let annot = self.parse_annot()?;
                                params.push((pn, annot));
                                match self.cur().tok {
                                    Tok::P(crate::lexer::P::Comma) => {
                                        self.bump();
                                    }
                                    Tok::P(crate::lexer::P::RP) => {}
                                    _ => {
                                        self.error(&self.cur().span, "expected ',' or ')'".into());
                                        return None;
                                    }
                                }
                            }
                            other => {
                                self.error(&self.cur().span, format!("expected parameter, found {}", tok_str(&other)));
                                return None;
                            }
                        }
                    }
                    let ret = if matches!(self.cur().tok, Tok::P(crate::lexer::P::Arrow)) {
                        self.bump();
                        Some(self.parse_annot()?)
                    } else {
                        None
                    };
                    if !matches!(self.cur().tok, Tok::Newline) {
                        self.error(&self.cur().span, "expected a newline after extern signature".into());
                        return None;
                    }
                    self.bump();
                    fns.push(ExternFn {
                        name,
                        params,
                        variadic,
                        ret,
                        span: fsp,
                    });
                }
                other => {
                    self.error(&self.cur().span, format!("expected 'func' in extern block, found {}", tok_str(&other)));
                    return None;
                }
            }
        }
        Some(fns)
    }

    fn parse_use(&mut self, sp: Span) -> Option<(String, Option<String>, Option<Vec<String>>)> {
        self.bump();
        let target = match self.cur().tok.clone() {
            Tok::Ident(n) => {
                self.bump();
                n
            }
            Tok::Str(parts) => {
                self.bump();
                match parts.as_slice() {
                    [StrPart::Lit(s)] => {
                        if !s.ends_with(".strc") {
                            self.error(&sp, "use paths must point to a .strc file".into());
                            return None;
                        }
                        s.clone()
                    }
                    _ => {
                        self.error(&sp, "use path must be a plain string".into());
                        return None;
                    }
                }
            }
            other => {
                self.error(&sp, format!("expected a module name or path string, found {}", tok_str(&other)));
                return None;
            }
        };
        let mut names = None;
        if matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
            self.bump();
            let mut list = Vec::new();
            loop {
                match self.cur().tok.clone() {
                    Tok::Ident(n) => {
                        self.bump();
                        list.push(n);
                        if matches!(self.cur().tok, Tok::P(crate::lexer::P::Comma)) {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    other => {
                        self.error(&self.cur().span, format!("expected a name to import, found {}", tok_str(&other)));
                        return None;
                    }
                }
            }
            names = Some(list);
        }
        let mut alias = None;
        if matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::As)) {
            self.bump();
            match self.cur().tok.clone() {
                Tok::Ident(n) => {
                    self.bump();
                    alias = Some(n);
                }
                other => {
                    self.error(&self.cur().span, format!("expected an alias name, found {}", tok_str(&other)));
                    return None;
                }
            }
        }
        if !matches!(self.cur().tok, Tok::Newline) && !self.at_eof() {
            self.error(&self.cur().span, "expected a newline after use".into());
            return None;
        }
        Some((target, alias, names))
    }

    fn parse_block(&mut self) -> Option<Vec<Stmt>> {
        if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
            self.error(&self.cur().span, "expected ':'".into());
            return None;
        }
        self.bump();
        if matches!(self.cur().tok, Tok::Newline) {
            self.bump();
            if !matches!(self.cur().tok, Tok::Indent) {
                self.error(&self.cur().span, "expected an indented block".into());
                return None;
            }
            self.bump();
            let mut out = Vec::new();
            self.depth += 1;
            loop {
                match self.cur().tok {
                    Tok::Dedent => {
                        self.bump();
                        break;
                    }
                    Tok::Eof => break,
                    Tok::Newline => {
                        self.bump();
                    }
                    _ => match self.parse_stmt(false, Vec::new()) {
                        Some(s) => out.push(s),
                        None => self.sync_stmt(),
                    },
                }
            }
            self.depth -= 1;
            Some(out)
        } else {
            let s = self.parse_simple_stmt()?;
            Some(vec![s])
        }
    }

    /// `match` is a soft keyword: it only starts a match statement when the next
    /// token is the subject variable (an identifier). Any other follow token
    /// (`=`, `.`, `[`, `(`, newline, ...) falls through to normal expression
    /// parsing, so variables named `match` keep working.
    fn parse_match(&mut self, sp: Span) -> Option<Stmt> {
        self.bump(); // 'match'
        let t = self.bump();
        let subject = match t.tok {
            Tok::Ident(n) => Expr::Ident(n, t.span.clone()),
            other => {
                self.error(&t.span, format!("match subject must be a variable, found {}", tok_str(&other)));
                return None;
            }
        };
        if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
            self.error(&self.cur().span, "expected ':' after match subject".into());
            return None;
        }
        self.bump();
        if !matches!(self.cur().tok, Tok::Newline) {
            self.error(&self.cur().span, "expected an indented block of 'case' arms".into());
            return None;
        }
        self.bump();
        if !matches!(self.cur().tok, Tok::Indent) {
            self.error(&self.cur().span, "expected an indented block of 'case' arms".into());
            return None;
        }
        self.bump();
        let mut arms = Vec::new();
        self.depth += 1;
        loop {
            match self.cur().tok.clone() {
                Tok::Dedent => {
                    self.bump();
                    break;
                }
                Tok::Eof => break,
                Tok::Newline => {
                    self.bump();
                }
                Tok::Ident(ref n) if n == "case" && self.case_ahead() => match self.parse_case() {
                    Some(a) => arms.push(a),
                    None => self.sync_stmt(),
                },
                other => {
                    self.error(&self.cur().span, format!("expected 'case' inside match body, found {}", tok_str(&other)));
                    return None;
                }
            }
        }
        self.depth -= 1;
        if arms.is_empty() {
            self.error(&sp, "match needs at least one 'case' arm".into());
            return None;
        }
        Some(Stmt::Match { subject, arms, span: sp })
    }

    /// A `case` token only starts an arm when the next token can begin a
    /// pattern, so variables named `case` still work inside match bodies.
    fn case_ahead(&self) -> bool {
        match &self.toks[(self.pos + 1).min(self.toks.len() - 1)].tok {
            Tok::Int(_) | Tok::Str(_) | Tok::P(crate::lexer::P::Minus) => true,
            Tok::Ident(n) => n == "_",
            Tok::P(crate::lexer::P::LP) | Tok::P(crate::lexer::P::LB) => true,
            _ => false,
        }
    }

    fn parse_case(&mut self) -> Option<MatchCase> {
        let sp = self.cur().span.clone();
        self.bump(); // 'case'
        let mut values: Vec<Expr> = Vec::new();
        let mut wildcard = false;
        loop {
            let t = self.bump();
            match t.tok {
                Tok::Ident(ref n) if n == "_" => {
                    wildcard = true;
                }
                Tok::Int(v) => {
                    values.push(Expr::IntLit(v, t.span.clone()));
                }
                Tok::Str(parts) => match parts.as_slice() {
                    [StrPart::Lit(s)] => values.push(Expr::StrLit(s.clone(), t.span.clone())),
                    _ => {
                        self.error(&t.span, "case patterns must be plain string literals".into());
                        return None;
                    }
                },
                Tok::P(crate::lexer::P::Minus) => {
                    let t2 = self.bump();
                    match t2.tok {
                        Tok::Int(v) => values.push(Expr::IntLit(v.wrapping_neg(), t2.span.clone())),
                        other => {
                            self.error(&t2.span, format!("expected an integer after '-', found {}", tok_str(&other)));
                            return None;
                        }
                    }
                }
                Tok::P(crate::lexer::P::LP) => {
                    let items = self.parse_pattern_seq(crate::lexer::P::RP, &t.span)?;
                    values.push(Expr::Tuple(items, t.span.clone()));
                }
                Tok::P(crate::lexer::P::LB) => {
                    let items = self.parse_pattern_seq(crate::lexer::P::RB, &t.span)?;
                    values.push(Expr::ListLit(items, t.span.clone()));
                }
                other => {
                    self.error(&t.span, format!("expected a literal or '_' in case pattern, found {}", tok_str(&other)));
                    return None;
                }
            }
            if matches!(self.cur().tok, Tok::P(crate::lexer::P::Comma)) {
                if wildcard {
                    self.error(&self.cur().span, "'_' cannot be combined with other patterns".into());
                    return None;
                }
                self.bump();
                continue;
            }
            break;
        }
        let body = self.parse_block()?;
        Some(MatchCase {
            values,
            wildcard,
            body,
            span: sp,
        })
    }

    /// Parse the comma-separated contents of a `(…)` or `[…]` case pattern,
    /// consuming through the closing bracket.
    fn parse_pattern_seq(&mut self, close: crate::lexer::P, open_sp: &Span) -> Option<Vec<Expr>> {
        let mut items = Vec::new();
        loop {
            if matches!(&self.cur().tok, Tok::P(p) if *p == close) {
                self.bump();
                break;
            }
            items.push(self.parse_pattern_el()?);
            if matches!(self.cur().tok, Tok::P(crate::lexer::P::Comma)) {
                self.bump();
                continue;
            }
            if matches!(&self.cur().tok, Tok::P(p) if *p == close) {
                self.bump();
                break;
            }
            self.error(&self.cur().span, "expected ',' or the closing bracket of the pattern".into());
            return None;
        }
        if items.is_empty() {
            self.error(open_sp, "pattern needs at least one element".into());
            return None;
        }
        Some(items)
    }

    /// One pattern element: a binding name, `_`, a literal, or a nested
    /// `(…)` / `[…]` pattern.
    fn parse_pattern_el(&mut self) -> Option<Expr> {
        let t = self.bump();
        match t.tok {
            Tok::Ident(n) => Some(Expr::Ident(n, t.span.clone())),
            Tok::Kw(crate::lexer::Kw::True) => Some(Expr::BoolLit(true, t.span.clone())),
            Tok::Kw(crate::lexer::Kw::False) => Some(Expr::BoolLit(false, t.span.clone())),
            Tok::Kw(crate::lexer::Kw::None) => {
                self.error(&t.span, "none is not supported in patterns yet".into());
                None
            }
            Tok::Int(v) => Some(Expr::IntLit(v, t.span.clone())),
            Tok::Float(v) => Some(Expr::FloatLit(v, t.span.clone())),
            Tok::Str(parts) => match parts.as_slice() {
                [StrPart::Lit(s)] => Some(Expr::StrLit(s.clone(), t.span.clone())),
                _ => {
                    self.error(&t.span, "case patterns must be plain string literals".into());
                    None
                }
            },
            Tok::P(crate::lexer::P::Minus) => {
                let t2 = self.bump();
                match t2.tok {
                    Tok::Int(v) => Some(Expr::IntLit(v.wrapping_neg(), t2.span.clone())),
                    Tok::Float(v) => Some(Expr::FloatLit(-v, t2.span.clone())),
                    other => {
                        self.error(&t2.span, format!("expected a number after '-', found {}", tok_str(&other)));
                        None
                    }
                }
            }
            Tok::P(crate::lexer::P::LP) => {
                let items = self.parse_pattern_seq(crate::lexer::P::RP, &t.span)?;
                Some(Expr::Tuple(items, t.span.clone()))
            }
            Tok::P(crate::lexer::P::LB) => {
                let items = self.parse_pattern_seq(crate::lexer::P::RB, &t.span)?;
                Some(Expr::ListLit(items, t.span.clone()))
            }
            other => {
                self.error(&t.span, format!("expected a name, literal or nested pattern, found {}", tok_str(&other)));
                None
            }
        }
    }

    fn parse_if(&mut self, sp: Span) -> Option<(Stmt, Span)> {
        self.bump();
        let cond = self.parse_expr_no_tuple()?;
        let body = self.parse_block()?;
        let mut arms = vec![(cond, body)];
        let mut else_body = None;
        loop {
            match self.cur().tok {
                Tok::Kw(crate::lexer::Kw::Elif) => {
                    self.bump();
                    let c = self.parse_expr_no_tuple()?;
                    let b = self.parse_block()?;
                    arms.push((c, b));
                }
                Tok::Kw(crate::lexer::Kw::Else) => {
                    self.bump();
                    else_body = Some(self.parse_block()?);
                    break;
                }
                _ => break,
            }
        }
        Some((
            Stmt::If {
                arms,
                else_body,
                span: sp.clone(),
            },
            sp,
        ))
    }

    fn parse_while(&mut self, sp: Span) -> Option<Stmt> {
        self.bump();
        let cond = self.parse_expr_no_tuple()?;
        let body = self.parse_block()?;
        Some(Stmt::While { cond, body, span: sp })
    }

    fn parse_for(&mut self, sp: Span) -> Option<Stmt> {
        self.bump();
        let mut vars = Vec::new();
        loop {
            match self.cur().tok.clone() {
                Tok::Ident(n) => {
                    self.bump();
                    vars.push(n);
                    if matches!(self.cur().tok, Tok::P(crate::lexer::P::Comma)) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                other => {
                    self.error(&sp, format!("expected a loop variable name, found {}", tok_str(&other)));
                    return None;
                }
            }
        }
        if !matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::In)) {
            self.error(&self.cur().span, "expected 'in' after loop variables".into());
            return None;
        }
        self.bump();
        let iter = self.parse_expr_no_tuple()?;
        let body = self.parse_block()?;
        if vars.len() > 2 {
            self.error(&sp, "for takes at most 2 variables (i, x)".into());
            return None;
        }
        Some(Stmt::For {
            vars,
            iter,
            body,
            span: sp,
        })
    }

    fn parse_simple_stmt(&mut self) -> Option<Stmt> {
        let sp = self.cur().span.clone();
        match self.cur().tok.clone() {
            Tok::Kw(crate::lexer::Kw::Pass) => {
                self.bump();
                self.end_stmt()?;
                Some(Stmt::Pass(sp))
            }
            Tok::Kw(crate::lexer::Kw::Global) => {
                self.bump();
                let mut names = Vec::new();
                loop {
                    match self.cur().tok.clone() {
                        Tok::Ident(n) => {
                            names.push(n);
                            self.bump();
                        }
                        _ => {
                            self.error(&self.cur().span, "expected a name after 'global'".into());
                            return None;
                        }
                    }
                    if matches!(self.cur().tok, Tok::P(crate::lexer::P::Comma)) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                self.end_stmt()?;
                Some(Stmt::Global(names, sp))
            }
            Tok::Kw(crate::lexer::Kw::Break) => {
                self.bump();
                self.end_stmt()?;
                Some(Stmt::Break(sp))
            }
            Tok::Kw(crate::lexer::Kw::Continue) => {
                self.bump();
                self.end_stmt()?;
                Some(Stmt::Continue(sp))
            }
            Tok::Kw(crate::lexer::Kw::Return) => {
                self.bump();
                let value = if matches!(self.cur().tok, Tok::Newline | Tok::Eof | Tok::Dedent) {
                    None
                } else {
                    Some(self.parse_expr_list()?)
                };
                self.end_stmt()?;
                Some(Stmt::Return { value, span: sp })
            }
            _ => {
                let first = self.parse_expr_list()?;
                if matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                    let name = match &first {
                        Expr::Ident(n, _) => n.clone(),
                        other => {
                            self.error(&other.span(), "only a plain name can have a type annotation".into());
                            return None;
                        }
                    };
                    self.bump();
                    let annot = self.parse_annot()?;
                    if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Eq)) {
                        self.error(&self.cur().span, "expected '= value' (Still figures out types on its own; write x = 5, or x: int = 5)".into());
                        return None;
                    }
                    self.bump();
                    let value = self.parse_expr_list()?;
                    self.end_stmt()?;
                    return Some(Stmt::Assign {
                        targets: vec![Expr::Ident(name, sp.clone())],
                        op: None,
                        value,
                        annot: Some(annot),
                        span: sp,
                    });
                }
                let op = match self.cur().tok {
                    Tok::P(crate::lexer::P::Eq) => None,
                    Tok::P(crate::lexer::P::PlusEq) => Some(BinOp::Add),
                    Tok::P(crate::lexer::P::MinusEq) => Some(BinOp::Sub),
                    Tok::P(crate::lexer::P::StarEq) => Some(BinOp::Mul),
                    Tok::P(crate::lexer::P::SlashEq) => Some(BinOp::Div),
                    Tok::P(crate::lexer::P::DSlashEq) => Some(BinOp::FloorDiv),
                    Tok::P(crate::lexer::P::DStarEq) => Some(BinOp::Pow),
                    _ => {
                        self.end_stmt()?;
                        return Some(Stmt::ExprStmt { expr: first, span: sp });
                    }
                };
                self.bump();
                let value = self.parse_expr_list()?;
                self.end_stmt()?;
                let targets = match first {
                    Expr::Tuple(items, _) => items,
                    other => vec![other],
                };
                for t in &targets {
                    match t {
                        Expr::Ident(..) | Expr::Index(..) | Expr::Member(..) => {}
                        Expr::Slice(..) => {
                            self.error(&t.span(), "cannot assign to a slice".into());
                            return None;
                        }
                        other => {
                            self.error(&other.span(), "invalid assignment target".into());
                            return None;
                        }
                    }
                }
                if op.is_some() && targets.len() != 1 {
                    self.error(&sp, "+= and friends need exactly one target".into());
                    return None;
                }
                Some(Stmt::Assign {
                    targets,
                    op,
                    value,
                    annot: None,
                    span: sp,
                })
            }
        }
    }

    fn end_stmt(&mut self) -> Option<()> {
        match self.cur().tok {
            Tok::Newline => {
                self.bump();
                Some(())
            }
            Tok::Eof | Tok::Dedent => Some(()),
            _ => {
                self.error(&self.cur().span, "expected a newline at the end of the statement".into());
                None
            }
        }
    }

    fn parse_expr_list(&mut self) -> Option<Expr> {
        let first = self.parse_ternary()?;
        if matches!(self.cur().tok, Tok::P(crate::lexer::P::Comma)) {
            let sp = first.span();
            let mut items = vec![first];
            while matches!(self.cur().tok, Tok::P(crate::lexer::P::Comma)) {
                self.bump();
                if matches!(self.cur().tok, Tok::Newline | Tok::Eof | Tok::P(crate::lexer::P::RP)) {
                    break;
                }
                items.push(self.parse_ternary()?);
            }
            return Some(Expr::Tuple(items, sp));
        }
        Some(first)
    }

    fn parse_expr_no_tuple(&mut self) -> Option<Expr> {
        self.parse_ternary()
    }

    /// After the slice-end part: if a ':' follows, parse the step (`a:b:c`).
    /// `[:]`/`[a::c]`/`[a:b:]` all yield None when the step slot is empty.
    fn parse_slice_step(&mut self) -> Option<Option<Box<Expr>>> {
        if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
            return Some(None);
        }
        self.bump();
        if matches!(self.cur().tok, Tok::P(crate::lexer::P::RB)) {
            return Some(None);
        }
        Some(Some(Box::new(self.parse_ternary()?)))
    }

    fn parse_ternary(&mut self) -> Option<Expr> {
        let val = self.parse_or()?;
        if matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::If)) {
            let sp = val.span();
            self.bump();
            let cond = self.parse_or()?;
            if !matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::Else)) {
                self.error(&self.cur().span, "expected 'else' in 'a if cond else b'".into());
                return None;
            }
            self.bump();
            let els = self.parse_ternary()?;
            return Some(Expr::Cond(Box::new(val), Box::new(cond), Box::new(els), sp));
        }
        Some(val)
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut l = self.parse_and()?;
        while matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::Or)) {
            let sp = l.span();
            self.bump();
            let r = self.parse_and()?;
            l = Expr::Or(Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut l = self.parse_not()?;
        while matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::And)) {
            let sp = l.span();
            self.bump();
            let r = self.parse_not()?;
            l = Expr::And(Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_not(&mut self) -> Option<Expr> {
        if matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::Not)) {
            let sp = self.cur().span.clone();
            self.bump();
            let inner = self.parse_not()?;
            return Some(Expr::Not(Box::new(inner), sp));
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Option<Expr> {
        let l = self.parse_bitor()?;
        let sp = l.span();
        let op = match self.cur().tok {
            Tok::P(crate::lexer::P::EqEq) => Some(CmpOp::Eq),
            Tok::P(crate::lexer::P::NotEq) => Some(CmpOp::Ne),

            Tok::P(crate::lexer::P::Lt) => Some(CmpOp::Lt),

            Tok::P(crate::lexer::P::Le) => Some(CmpOp::Le),

            Tok::P(crate::lexer::P::Gt) => Some(CmpOp::Gt),

            Tok::P(crate::lexer::P::Ge) => Some(CmpOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let r = self.parse_add()?;
            if matches!(
                self.cur().tok,
                Tok::P(crate::lexer::P::EqEq)
                    | Tok::P(crate::lexer::P::NotEq)
                    | Tok::P(crate::lexer::P::Lt)
                    | Tok::P(crate::lexer::P::Le)
                    | Tok::P(crate::lexer::P::Gt)
                    | Tok::P(crate::lexer::P::Ge)
                    | Tok::Kw(crate::lexer::Kw::In)
                    | Tok::Kw(crate::lexer::Kw::Is)
            ) {
                self.error(&self.cur().span, "chained comparisons are not supported; combine them with and (a < b and b < c)".into());
                return None;
            }
            return Some(Expr::Cmp(Box::new(l), op, Box::new(r), sp));
        }
        match self.cur().tok {
            Tok::Kw(crate::lexer::Kw::In) => {
                self.bump();
                let r = self.parse_add()?;
                Some(Expr::In(Box::new(l), Box::new(r), false, sp))
            }
            Tok::Kw(crate::lexer::Kw::Not) => {
                self.bump();
                if !matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::In)) {
                    self.error(&self.cur().span, "expected 'in' after 'not' here".into());
                    return None;
                }
                self.bump();
                let r = self.parse_add()?;
                Some(Expr::In(Box::new(l), Box::new(r), true, sp))
            }
            Tok::Kw(crate::lexer::Kw::Is) => {
                self.bump();
                let neg = if matches!(self.cur().tok, Tok::Kw(crate::lexer::Kw::Not)) {
                    self.bump();
                    true
                } else {
                    false
                };
                let r = self.parse_add()?;
                match r {
                    Expr::NoneLit(_) => Some(Expr::IsNone(Box::new(l), neg, sp)),
                    other => {
                        self.error(&other.span(), "'is' only works with none (x is none / x is not none)".into());
                        None
                    }
                }
            }
            _ => Some(l),
        }
    }

    fn parse_bitor(&mut self) -> Option<Expr> {
        let mut l = self.parse_bitxor()?;
        loop {
            let op = match self.cur().tok {
                Tok::P(crate::lexer::P::Bor) => BinOp::Bor,
                _ => break,
            };
            let sp = l.span();
            self.bump();
            let r = self.parse_bitxor()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_bitxor(&mut self) -> Option<Expr> {
        let mut l = self.parse_bitand()?;
        loop {
            let op = match self.cur().tok {
                Tok::P(crate::lexer::P::Bxor) => BinOp::Bxor,
                _ => break,
            };
            let sp = l.span();
            self.bump();
            let r = self.parse_bitand()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_bitand(&mut self) -> Option<Expr> {
        let mut l = self.parse_bitshift()?;
        loop {
            let op = match self.cur().tok {
                Tok::P(crate::lexer::P::Band) => BinOp::Band,
                _ => break,
            };
            let sp = l.span();
            self.bump();
            let r = self.parse_bitshift()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_bitshift(&mut self) -> Option<Expr> {
        let mut l = self.parse_add()?;
        loop {
            let op = match self.cur().tok {
                Tok::P(crate::lexer::P::LSL) => BinOp::Shl,
                Tok::P(crate::lexer::P::RSr) => BinOp::Shr,
                _ => break,
            };
            let sp = l.span();
            self.bump();
            let r = self.parse_add()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_add(&mut self) -> Option<Expr> {
        let mut l = self.parse_mul()?;
        loop {
            let op = match self.cur().tok {
                Tok::P(crate::lexer::P::Plus) => BinOp::Add,
                Tok::P(crate::lexer::P::Minus) => BinOp::Sub,
                _ => break,
            };
            let sp = l.span();
            self.bump();
            let r = self.parse_mul()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_mul(&mut self) -> Option<Expr> {
        let mut l = self.parse_unary()?;
        loop {
            let op = match self.cur().tok {
                Tok::P(crate::lexer::P::Star) => BinOp::Mul,
                Tok::P(crate::lexer::P::Slash) => BinOp::Div,
                Tok::P(crate::lexer::P::DSlash) => BinOp::FloorDiv,
                Tok::P(crate::lexer::P::Percent) => BinOp::Mod,
                _ => break,
            };
            let sp = l.span();
            self.bump();
            let r = self.parse_unary()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), sp);
        }
        Some(l)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if matches!(self.cur().tok, Tok::P(crate::lexer::P::Minus)) {
            let sp = self.cur().span.clone();
            self.bump();
            let inner = self.parse_unary()?;
            return Some(Expr::Unary(UnOp::Neg, Box::new(inner), sp));
        }
        if matches!(self.cur().tok, Tok::P(crate::lexer::P::Tilde)) {
            let sp = self.cur().span.clone();
            self.bump();
            let inner = self.parse_unary()?;
            return Some(Expr::Unary(UnOp::BNot, Box::new(inner), sp));
        }
        self.parse_pow()
    }

    fn parse_pow(&mut self) -> Option<Expr> {
        let base = self.parse_postfix()?;
        if matches!(self.cur().tok, Tok::P(crate::lexer::P::DStar)) {
            let sp = base.span();
            self.bump();
            let rhs = self.parse_unary()?;
            return Some(Expr::Binary(BinOp::Pow, Box::new(base), Box::new(rhs), sp));
        }
        Some(base)
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            match self.cur().tok {
                Tok::P(crate::lexer::P::LP) => {
                    let sp = e.span();
                    self.bump();
                    let mut args = Vec::new();
                    loop {
                        match self.cur().tok {
                            Tok::P(crate::lexer::P::RP) => {
                                self.bump();
                                break;
                            }
                            Tok::Newline => {
                                self.bump();
                            }
                            _ => {
                                args.push(self.parse_ternary()?);
                                match self.cur().tok {
                                    Tok::P(crate::lexer::P::Comma) => {
                                        self.bump();
                                    }
                                    Tok::P(crate::lexer::P::RP) => {}
                                    Tok::Newline => {}
                                    _ => {
                                        self.error(&self.cur().span, "expected ',' or ')' in call".into());
                                        return None;
                                    }
                                }
                            }
                        }
                    }
                    e = Expr::Call(Box::new(e), args, sp);
                }
                Tok::P(crate::lexer::P::LB) => {
                    let sp = e.span();
                    self.bump();
                    // slice/index ladder: [i] | [a:b] | [a:b:c] | [:b] | [::c] | [a::c] | [:]
                    let mut start = None::<Box<Expr>>;
                    let mut sliced = false;
                    if matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                        self.bump();
                        sliced = true;
                    } else {
                        let idx = self.parse_ternary()?;
                        if matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                            start = Some(Box::new(idx));
                            self.bump();
                            sliced = true;
                        } else {
                            if !matches!(self.cur().tok, Tok::P(crate::lexer::P::RB)) {
                                self.error(&self.cur().span, "expected ']'".into());
                                return None;
                            }
                            self.bump();
                            e = Expr::Index(Box::new(e), Box::new(idx), sp.clone());
                        }
                    }
                    if sliced {
                        let end = if matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon) | Tok::P(crate::lexer::P::RB)) {
                            None
                        } else {
                            Some(Box::new(self.parse_ternary()?))
                        };
                        let step = self.parse_slice_step()?;
                        if !matches!(self.cur().tok, Tok::P(crate::lexer::P::RB)) {
                            self.error(&self.cur().span, "expected ']'".into());
                            return None;
                        }
                        self.bump();
                        e = Expr::Slice(Box::new(e), start, end, step, sp);
                    }
                }
                Tok::P(crate::lexer::P::Dot) => {
                    self.bump();
                    match self.cur().tok.clone() {
                        Tok::Ident(n) => {
                            let sp = e.span();
                            self.bump();
                            e = Expr::Member(Box::new(e), n, sp);
                        }
                        Tok::Int(i) => {
                            let sp = e.span();
                            self.bump();
                            e = Expr::Index(Box::new(e), Box::new(Expr::IntLit(i, sp.clone())), sp);
                        }
                        other => {
                            self.error(&self.cur().span, format!("expected a field name after '.', found {}", tok_str(&other)));
                            return None;
                        }
                    }
                }
                _ => break,
            }
        }
        Some(e)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        self.expr_depth += 1;
        if self.expr_depth > 100 {
            self.expr_depth -= 1;
            self.error(&self.cur().span, "expression is nested too deeply".into());
            return None;
        }
        let r = self.parse_primary_inner();
        self.expr_depth -= 1;
        r
    }

    fn parse_primary_inner(&mut self) -> Option<Expr> {
        let sp = self.cur().span.clone();
        match self.cur().tok.clone() {
            Tok::Int(v) => {
                self.bump();
                Some(Expr::IntLit(v, sp))
            }
            Tok::Float(v) => {
                self.bump();
                Some(Expr::FloatLit(v, sp))
            }
            Tok::Kw(crate::lexer::Kw::True) => {
                self.bump();
                Some(Expr::BoolLit(true, sp))
            }
            Tok::Kw(crate::lexer::Kw::False) => {
                self.bump();
                Some(Expr::BoolLit(false, sp))
            }
            Tok::Kw(crate::lexer::Kw::None) => {
                self.bump();
                Some(Expr::NoneLit(sp))
            }
            Tok::Str(parts) => {
                self.bump();
                self.build_str(parts, sp)
            }
            Tok::Ident(n) => {
                self.bump();
                Some(Expr::Ident(n, sp))
            }
            Tok::P(crate::lexer::P::LP) => {
                self.bump();
                if matches!(self.cur().tok, Tok::P(crate::lexer::P::RP)) {
                    self.bump();
                    return Some(Expr::Tuple(Vec::new(), sp));
                }
                let e = self.parse_expr_list()?;
                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::RP)) {
                    self.error(&self.cur().span, "expected ')'".into());
                    return None;
                }
                self.bump();
                Some(e)
            }
            Tok::P(crate::lexer::P::LB) => {
                self.bump();
                let mut items = Vec::new();
                loop {
                    match self.cur().tok {
                        Tok::P(crate::lexer::P::RB) => {
                            self.bump();
                            break;
                        }
                        Tok::Newline => {
                            self.bump();
                        }
                        _ => {
                            items.push(self.parse_ternary()?);
                            match self.cur().tok {
                                Tok::P(crate::lexer::P::Comma) => {
                                    self.bump();
                                }
                                Tok::P(crate::lexer::P::RB) => {}
                                Tok::Newline => {}
                                _ => {
                                    self.error(&self.cur().span, "expected ',' or ']' in list".into());
                                    return None;
                                }
                            }
                        }
                    }
                }
                Some(Expr::ListLit(items, sp))
            }
            Tok::P(crate::lexer::P::LC) => {
                self.bump();
                let mut pairs = Vec::new();
                loop {
                    match self.cur().tok {
                        Tok::P(crate::lexer::P::RC) => {
                            self.bump();
                            break;
                        }
                        Tok::Newline => {
                            self.bump();
                        }
                        _ => {
                            let k = self.parse_ternary()?;
                            if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                                self.error(&self.cur().span, "expected ':' in dict entry".into());
                                return None;
                            }
                            self.bump();
                            let v = self.parse_ternary()?;
                            pairs.push((k, v));
                            match self.cur().tok {
                                Tok::P(crate::lexer::P::Comma) => {
                                    self.bump();
                                }
                                Tok::P(crate::lexer::P::RC) => {}
                                Tok::Newline => {}
                                _ => {
                                    self.error(&self.cur().span, "expected ',' or '}' in dict".into());
                                    return None;
                                }
                            }
                        }
                    }
                }
                Some(Expr::DictLit(pairs, sp))
            }
            other => {
                self.error(&sp, format!("expected an expression, found {}", tok_str(&other)));
                None
            }
        }
    }

    fn build_str(&mut self, parts: Vec<StrPart>, sp: Span) -> Option<Expr> {
        if parts.iter().all(|p| matches!(p, StrPart::Lit(_))) {
            let joined: String = parts
                .iter()
                .map(|p| match p {
                    StrPart::Lit(s) => s.clone(),
                    _ => String::new(),
                })
                .collect();
            return Some(Expr::StrLit(joined, sp));
        }
        let mut out = Vec::new();
        for p in parts {
            match p {
                StrPart::Lit(s) => out.push(InterpPart::Lit(s)),
                StrPart::Code(code, csp) => {
                    let e = self.parse_interp_code(&code, &csp)?;
                    out.push(InterpPart::Code(e));
                }
            }
        }
        Some(Expr::Interp(out, sp))
    }

    fn parse_interp_code(&mut self, code: &str, sp: &Span) -> Option<Expr> {
        let (toks, sub_diags) = Lexer::new(code, &sp.file).run();
        let mut fixed: Vec<Token> = Vec::new();
        for mut t in toks {
            if t.span.line == 1 {
                t.span.col += sp.col;
            }
            t.span.line += sp.line - 1;
            match t.tok {
                Tok::Indent => continue,
                Tok::Dedent => continue,
                Tok::Newline => continue,
                _ => fixed.push(t),
            }
        }
        self.diags.extend(sub_diags);
        let mut sub = Parser {
            toks: fixed,
            pos: 0,
            diags: Vec::new(),
            file: self.file.clone(),
            depth: 0,
            expr_depth: 0,
        };
        let e = sub.parse_ternary();
        if !sub.at_eof() {
            sub.error(&sub.cur().span, "unexpected trailing tokens inside {{...}}".into());
        }
        match e {
            Some(e) if sub.diags.is_empty() => Some(e),
            Some(e) => {
                self.diags.extend(sub.diags);
                Some(e)
            }
            None => {
                self.diags.extend(sub.diags);
                self.error(sp, "invalid expression inside string interpolation".into());
                None
            }
        }
    }

    fn parse_annot(&mut self) -> Option<TypeAnnot> {
        let base = match self.cur().tok.clone() {
            Tok::Ident(n) => {
                self.bump();
                match n.as_str() {
                    "int" => TypeAnnot::Int,
                    "float" => TypeAnnot::Float,
                    "bool" => TypeAnnot::Bool,
                    "str" => TypeAnnot::Str,
                    "none" => TypeAnnot::None,
                    "any" => TypeAnnot::Any,
                    _ => TypeAnnot::Named(n),
                }
            }
            Tok::P(crate::lexer::P::LB) => {
                self.bump();
                let inner = self.parse_annot()?;
                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::RB)) {
                    self.error(&self.cur().span, "expected ']' in list type".into());
                    return None;
                }
                self.bump();
                TypeAnnot::List(Box::new(inner))
            }
            Tok::P(crate::lexer::P::LC) => {
                self.bump();
                let k = self.parse_annot()?;
                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::Colon)) {
                    self.error(&self.cur().span, "expected ':' in dict type".into());
                    return None;
                }
                self.bump();
                let v = self.parse_annot()?;
                if !matches!(self.cur().tok, Tok::P(crate::lexer::P::RC)) {
                    self.error(&self.cur().span, "expected '}' in dict type".into());
                    return None;
                }
                self.bump();
                TypeAnnot::Dict(Box::new(k), Box::new(v))
            }
            Tok::P(crate::lexer::P::LP) => {
                self.bump();
                let mut parts = Vec::new();
                loop {
                    if matches!(self.cur().tok, Tok::P(crate::lexer::P::RP)) {
                        self.bump();
                        break;
                    }
                    parts.push(self.parse_annot()?);
                    match self.cur().tok {
                        Tok::P(crate::lexer::P::Comma) => {
                            self.bump();
                        }
                        Tok::P(crate::lexer::P::RP) => {}
                        _ => {
                            self.error(&self.cur().span, "expected ',' or ')' in tuple type".into());
                            return None;
                        }
                    }
                }
                if parts.len() == 1 {
                    parts.pop().unwrap()
                } else {
                    TypeAnnot::Tuple(parts)
                }
            }
            other => {
                self.error(&self.cur().span, format!("expected a type, found {}", tok_str(&other)));
                return None;
            }
        };
        if matches!(self.cur().tok, Tok::P(crate::lexer::P::Question)) {
            self.bump();
            return Some(TypeAnnot::Nullable(Box::new(base)));
        }
        Some(base)
    }
}

fn tok_str(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => format!("'{}'", s),
        Tok::Int(v) => format!("number {}", v),
        Tok::Float(v) => format!("float {}", v),
        Tok::Str(_) => "a string".into(),
        Tok::Kw(k) => format!("'{}'", kw_str(k)),
        Tok::P(p) => format!("'{}'", p_str(p)),
        Tok::Newline => "end of line".into(),
        Tok::Indent => "an indentation".into(),
        Tok::Dedent => "a dedent".into(),
        Tok::Err(_) => "an invalid token".into(),
        Tok::Eof => "end of file".into(),
    }
}

pub fn kw_str(k: &crate::lexer::Kw) -> &'static str {
    use crate::lexer::Kw::*;
    match k {
        Func => "func",
        Type => "type",
        Extern => "extern",
        Use => "use",
        If => "if",
        Elif => "elif",
        Else => "else",
        While => "while",
        For => "for",
        In => "in",
        And => "and",
        Or => "or",
        Not => "not",
        Is => "is",
        Break => "break",
        Continue => "continue",
        Return => "return",
        True => "true",
        False => "false",
        None => "none",
        Pass => "pass",
        As => "as",
        Global => "global",
    }
}

pub fn p_str(p: &crate::lexer::P) -> &'static str {
    use crate::lexer::P::*;
    match p {
        LP => "(",
        RP => ")",
        LB => "[",
        RB => "]",
        LC => "{",
        RC => "}",
        Comma => ",",
        Colon => ":",
        Dot => ".",
        Arrow => "->",
        Plus => "+",
        Minus => "-",
        Star => "*",
        Slash => "/",
        DSlash => "//",
        Percent => "%",
        DStar => "**",
        Eq => "=",
        PlusEq => "+=",
        MinusEq => "-=",
        StarEq => "*=",
        SlashEq => "/=",
        DSlashEq => "//=",
        DStarEq => "**=",
        EqEq => "==",
        NotEq => "!=",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        LSL => "<<",
        RSr => ">>",
        Band => "&",
        Bor => "|",
        Bxor => "^",
        Tilde => "~",
        Question => "?",
        At => "@",
    }
}
