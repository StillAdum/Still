//! Language Server Protocol implementation for Still.
//!
//! `still lsp` speaks JSON-RPC 2.0 with Content-Length framing on stdio —
//! the standard editor-server protocol, so Neovim/Emacs/other LSP clients get
//! the same diagnostics/symbols/completion the VSCode extension has, without
//! needing a client-side plugin.
//!
//! Scope (v0.3, honest): full-sync text documents, publishDiagnostics on
//! open/change/close, documentSymbol, hover for known top-level symbols,
//! completion (keywords + file symbols). No formatting, no rename, no
//! cross-file references yet.

use crate::span::{Diag, Span};
use std::collections::HashMap;
use std::io::{BufRead, Write};

// ---------------------------------------------------------------- tiny JSON

/// Minimal JSON value. Objects keep insertion order (LSP is tolerant, but
/// stable output makes tests deterministic).
#[derive(Clone, Debug, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            J::Num(n) => Some(*n),
            _ => None,
        }
    }
}

pub fn jstr(s: &str) -> J {
    J::Str(s.to_string())
}

pub fn jnum(n: i64) -> J {
    J::Num(n as f64)
}

fn jser(v: &J, out: &mut String) {
    match v {
        J::Null => out.push_str("null"),
        J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        J::Num(n) => {
            if n.fract() == 0.0 && n.abs() < 9.007199254740992e15 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{}", n));
            }
        }
        J::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        J::Arr(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                jser(it, out);
            }
            out.push(']');
        }
        J::Obj(pairs) => {
            out.push('{');
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                jser(&J::Str(k.clone()), out);
                out.push(':');
                jser(v, out);
            }
            out.push('}');
        }
    }
}

pub fn jtostr(v: &J) -> String {
    let mut s = String::new();
    jser(v, &mut s);
    s
}

pub fn jparse(text: &str) -> Option<J> {
    let mut p = JParser {
        b: text.chars().collect(),
        i: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return None; // trailing garbage
    }
    Some(v)
}

struct JParser {
    b: Vec<char>,
    i: usize,
}

impl JParser {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], ' ' | '\t' | '\n' | '\r') {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<char> {
        self.b.get(self.i).copied()
    }
    fn value(&mut self) -> Option<J> {
        self.ws();
        match self.peek()? {
            '{' => self.obj(),
            '[' => self.arr(),
            '"' => self.string().map(J::Str),
            't' => self.lit("true", J::Bool(true)),
            'f' => self.lit("false", J::Bool(false)),
            'n' => self.lit("null", J::Null),
            _ => self.number(),
        }
    }
    fn lit(&mut self, word: &str, v: J) -> Option<J> {
        for c in word.chars() {
            if self.peek()? != c {
                return None;
            }
            self.i += 1;
        }
        Some(v)
    }
    fn obj(&mut self) -> Option<J> {
        self.i += 1; // {
        let mut pairs = Vec::new();
        self.ws();
        if self.peek()? == '}' {
            self.i += 1;
            return Some(J::Obj(pairs));
        }
        loop {
            self.ws();
            let k = self.string()?;
            self.ws();
            if self.peek()? != ':' {
                return None;
            }
            self.i += 1;
            let v = self.value()?;
            pairs.push((k, v));
            self.ws();
            match self.peek()? {
                ',' => {
                    self.i += 1;
                }
                '}' => {
                    self.i += 1;
                    return Some(J::Obj(pairs));
                }
                _ => return None,
            }
        }
    }
    fn arr(&mut self) -> Option<J> {
        self.i += 1; // [
        let mut items = Vec::new();
        self.ws();
        if self.peek()? == ']' {
            self.i += 1;
            return Some(J::Arr(items));
        }
        loop {
            let v = self.value()?;
            items.push(v);
            self.ws();
            match self.peek()? {
                ',' => {
                    self.i += 1;
                }
                ']' => {
                    self.i += 1;
                    return Some(J::Arr(items));
                }
                _ => return None,
            }
        }
    }
    fn string(&mut self) -> Option<String> {
        if self.peek()? != '"' {
            return None;
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = self.peek()?;
            self.i += 1;
            match c {
                '"' => return Some(out),
                '\\' => {
                    let e = self.peek()?;
                    self.i += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code) {
                                // surrogate pair
                                if self.peek() == Some('\\') {
                                    self.i += 1;
                                    if self.peek() == Some('u') {
                                        self.i += 1;
                                        let lo = self.hex4()?;
                                        if (0xDC00..0xE000).contains(&lo) {
                                            code = 0x10000 + ((code - 0xD800) << 10) + (lo - 0xDC00);
                                        }
                                    }
                                }
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        _ => return None,
                    }
                }
                c => out.push(c),
            }
        }
    }
    fn hex4(&mut self) -> Option<u32> {
        let mut code = 0u32;
        for _ in 0..4 {
            let h = self.peek()?.to_digit(16)?;
            self.i += 1;
            code = code * 16 + h;
        }
        Some(code)
    }
    fn number(&mut self) -> Option<J> {
        let start = self.i;
        while self.i < self.b.len() && matches!(self.b[self.i], '0'..='9' | '-' | '+' | '.' | 'e' | 'E') {
            self.i += 1;
        }
        if start == self.i {
            return None;
        }
        let s: String = self.b[start..self.i].iter().collect();
        s.parse::<f64>().ok().map(J::Num)
    }
}

// ---------------------------------------------------------------- framing

fn read_message(r: &mut impl BufRead) -> Option<Vec<u8>> {
    let mut len: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        let n = r.read_line(&mut line).ok()?;
        if n == 0 {
            return None; // EOF
        }
        let t = line.trim_end();
        if t.is_empty() {
            break;
        }
        if let Some(rest) = t.strip_prefix("Content-Length:") {
            len = rest.trim().parse().ok();
        }
    }
    let n = len?;
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn write_frame(w: &mut impl Write, body: &str) {
    let _ = write!(w, "Content-Length: {}\r\n\r\n", body.len());
    let _ = w.write_all(body.as_bytes());
    let _ = w.flush();
}

fn send(w: &mut impl Write, id: J, result: &J) {
    let body = jtostr(&J::Obj(vec![
        ("jsonrpc".into(), jstr("2.0")),
        ("id".into(), id),
        ("result".into(), result.clone()),
    ]));
    write_frame(w, &body);
}

fn send_error(w: &mut impl Write, id: J, code: i64, message: &str) {
    let body = jtostr(&J::Obj(vec![
        ("jsonrpc".into(), jstr("2.0")),
        ("id".into(), id),
        (
            "error".into(),
            J::Obj(vec![("code".into(), jnum(code)), ("message".into(), jstr(message))]),
        ),
    ]));
    write_frame(w, &body);
}

fn notify(w: &mut impl Write, method: &str, params: &J) {
    let body = jtostr(&J::Obj(vec![
        ("jsonrpc".into(), jstr("2.0")),
        ("method".into(), jstr(method)),
        ("params".into(), params.clone()),
    ]));
    write_frame(w, &body);
}

// ---------------------------------------------------------------- positions

/// file:///C:/dir/x.strc -> C:/dir/x.strc ; file:///home/x -> /home/x
pub fn uri_to_path(uri: &str) -> String {
    let s = uri.strip_prefix("file://").unwrap_or(uri);
    let b = s.as_bytes();
    let s = if b.len() > 2 && b[0] == b'/' && b[2] == b':' { &s[1..] } else { s };
    s.replace("%20", " ").replace("%5C", "\\").replace("%3A", ":")
}

/// Span (1-based line, 1-based char col) -> LSP range (0-based lines, UTF-16
/// columns). The end is extended over the identifier under the diagnostic so
/// editors render a squiggle (Still spans are points).
fn lsp_range(text: &str, line1: u32, col1: u32) -> (u32, u32, u32, u32) {
    let lines: Vec<&str> = text.split('\n').collect();
    let li = (line1 as usize).saturating_sub(1).min(lines.len().saturating_sub(1));
    let line = lines[li];
    let chars: Vec<char> = line.chars().collect();
    let ci = (col1 as usize).saturating_sub(1).min(chars.len());
    let mut ce = ci;
    while ce < chars.len() && (chars[ce].is_alphanumeric() || chars[ce] == '_') {
        ce += 1;
    }
    if ce == ci {
        ce = (ci + 1).min(chars.len());
    }
    let u16off = |n: usize| chars[..n.min(chars.len())].iter().map(|c| c.len_utf16() as u32).sum::<u32>();
    (li as u32, u16off(ci), li as u32, u16off(ce))
}

fn range_json(text: &str, line1: u32, col1: u32) -> J {
    let (sl, sc, el, ec) = lsp_range(text, line1, col1);
    J::Obj(vec![
        (
            "start".into(),
            J::Obj(vec![("line".into(), jnum(sl as i64)), ("character".into(), jnum(sc as i64))]),
        ),
        (
            "end".into(),
            J::Obj(vec![("line".into(), jnum(el as i64)), ("character".into(), jnum(ec as i64))]),
        ),
    ])
}

fn diag_json(d: &Diag, text: &str) -> J {
    J::Obj(vec![
        ("range".into(), range_json(text, d.span.line, d.span.col)),
        ("severity".into(), jnum(1)),
        ("source".into(), jstr("still")),
        ("message".into(), jstr(&d.msg)),
    ])
}

/// 0-based LSP position -> identifier containing it.
fn word_at(text: &str, line0: u32, ch0: u32) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let line = lines.get(line0 as usize).copied().unwrap_or("");
    let chars: Vec<char> = line.chars().collect();
    let mut ci = 0usize;
    let mut acc = 0u32;
    while ci < chars.len() && acc < ch0 {
        acc += chars[ci].len_utf16() as u32;
        ci += 1;
    }
    let mut s = ci;
    while s > 0 && (chars[s - 1].is_alphanumeric() || chars[s - 1] == '_') {
        s -= 1;
    }
    let mut e = ci;
    while e < chars.len() && (chars[e].is_alphanumeric() || chars[e] == '_') {
        e += 1;
    }
    chars[s..e].iter().collect()
}

fn range_of_line(text: &str, line1: u32) -> J {
    range_json(text, line1, 1)
}

// ---------------------------------------------------------------- symbols

const KEYWORDS: &[&str] = &[
    "func", "type", "extern", "use", "if", "elif", "else", "while", "for", "in", "and", "or",
    "not", "is", "break", "continue", "return", "true", "false", "none", "pass", "as", "global",
    "match", "case",
];

fn symbol_kind(kind: &str) -> i64 {
    // LSP SymbolKind
    match kind {
        "function" => 12,
        "type" => 5,    // Class is the closest for named types
        "method" => 6,
        "field" => 8,
        "variable" => 13,
        _ => 2, // module
    }
}

fn completion_kind(kind: &str) -> i64 {
    // LSP CompletionItemKind
    match kind {
        "function" | "method" => 3,
        "type" => 7,
        "variable" | "field" => 6,
        _ => 9, // module
    }
}

fn hover_markdown(kind: &str, name: &str, detail: &str) -> String {
    match kind {
        "function" | "method" => format!("```still\n{}({})\n```", name, detail),
        "type" => format!("```still\ntype {}\n```", name),
        "module" => format!("```still\nuse {}\n```", detail),
        "field" => format!("`{}.{}`", detail, name),
        _ => format!("`{}`", name),
    }
}

// ---------------------------------------------------------------- server

struct Doc {
    text: String,
}

fn check_doc(text: &str, path: &str) -> Vec<Diag> {
    let p = std::path::PathBuf::from(path);
    match crate::check_text(text, &p) {
        Ok(o) => o.diags,
        Err(e) => vec![Diag {
            span: Span::new(std::rc::Rc::from(path), 1, 1),
            msg: e,
        }],
    }
}

fn publish(w: &mut impl Write, uri: &str, text: &str) {
    let path = uri_to_path(uri);
    let diags = check_doc(text, &path);
    let items: Vec<J> = diags.iter().map(|d| diag_json(d, text)).collect();
    let params = J::Obj(vec![("uri".into(), jstr(uri)), ("diagnostics".into(), J::Arr(items))]);
    notify(w, "textDocument/publishDiagnostics", &params);
}

/// Run the server on stdio. Returns the process exit code.
pub fn serve() -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut rin = stdin.lock();
    let mut wout = stdout.lock();
    let mut docs: HashMap<String, Doc> = HashMap::new();
    let mut shutdown = false;
    loop {
        let msg = match read_message(&mut rin) {
            Some(m) => m,
            None => break,
        };
        let text = match String::from_utf8(msg) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let j = match jparse(&text) {
            Some(j) => j,
            None => continue,
        };
        let id = j.get("id").cloned();
        let method = j.get("method").and_then(|m| m.as_str()).unwrap_or("").to_string();
        let params = j.get("params").cloned().unwrap_or(J::Null);
        let wants_reply = id.is_some() && !matches!(method.as_str(), "exit");
        match method.as_str() {
            "initialize" => {
                let capabilities = J::Obj(vec![
                    ("textDocumentSync".into(), jnum(1)), // full sync
                    ("documentSymbolProvider".into(), J::Bool(true)),
                    ("hoverProvider".into(), J::Bool(true)),
                    (
                        "completionProvider".into(),
                        J::Obj(vec![("triggerCharacters".into(), J::Arr(vec![jstr(".")]))]),
                    ),
                ]);
                let server_info = J::Obj(vec![
                    ("name".into(), jstr("still")),
                    ("version".into(), jstr(crate::VERSION)),
                ]);
                send(
                    &mut wout,
                    id.unwrap_or(J::Null),
                    &J::Obj(vec![("capabilities".into(), capabilities), ("serverInfo".into(), server_info)]),
                );
            }
            "initialized" => {}
            "shutdown" => {
                shutdown = true;
                send(&mut wout, id.unwrap_or(J::Null), &J::Null);
            }
            "exit" => break,
            "textDocument/didOpen" => {
                let td = params.get("textDocument");
                let uri = td.and_then(|t| t.get("uri")).and_then(|u| u.as_str()).unwrap_or("").to_string();
                let text = td.and_then(|t| t.get("text")).and_then(|t| t.as_str()).unwrap_or("").to_string();
                if !uri.is_empty() {
                    docs.insert(uri.clone(), Doc { text: text.clone() });
                    publish(&mut wout, &uri, &text);
                }
            }
            "textDocument/didChange" => {
                let td = params.get("textDocument");
                let uri = td.and_then(|t| t.get("uri")).and_then(|u| u.as_str()).unwrap_or("").to_string();
                if let Some(J::Arr(changes)) = params.get("contentChanges") {
                    if let Some(new_text) = changes.last().and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                        docs.insert(uri.clone(), Doc { text: new_text.to_string() });
                        publish(&mut wout, &uri, new_text);
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = params.get("textDocument").and_then(|t| t.get("uri")).and_then(|u| u.as_str()) {
                    docs.remove(uri);
                    let params = J::Obj(vec![("uri".into(), jstr(uri)), ("diagnostics".into(), J::Arr(vec![]))]);
                    notify(&mut wout, "textDocument/publishDiagnostics", &params);
                }
            }
            "textDocument/documentSymbol" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let text = docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
                let path = uri_to_path(uri);
                let items: Vec<J> = crate::file_symbols(&text, &path)
                    .iter()
                    .map(|s| {
                        J::Obj(vec![
                            ("name".into(), jstr(&s.name)),
                            ("kind".into(), jnum(symbol_kind(s.kind))),
                            (
                                "location".into(),
                                J::Obj(vec![
                                    ("uri".into(), jstr(uri)),
                                    ("range".into(), range_of_line(&text, s.line.max(1))),
                                ]),
                            ),
                        ])
                    })
                    .collect();
                send(&mut wout, id.unwrap_or(J::Null), &J::Arr(items));
            }
            "textDocument/hover" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let pos = params.get("position").cloned().unwrap_or(J::Null);
                let line0 = pos.get("line").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
                let ch0 = pos.get("character").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
                let text = docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
                let word = word_at(&text, line0, ch0);
                let mut hover: Option<J> = None;
                if !word.is_empty() {
                    let path = uri_to_path(uri);
                    if let Some(s) = crate::file_symbols(&text, &path).iter().find(|s| s.name == word) {
                        hover = Some(J::Obj(vec![(
                            "contents".into(),
                            J::Obj(vec![("kind".into(), jnum(3)), ("value".into(), jstr(&hover_markdown(s.kind, &s.name, &s.detail)))]),
                        )]));
                    }
                }
                send(&mut wout, id.unwrap_or(J::Null), &hover.unwrap_or(J::Null));
            }
            "textDocument/completion" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let text = docs.get(uri).map(|d| d.text.clone()).unwrap_or_default();
                let path = uri_to_path(uri);
                let mut items: Vec<J> = KEYWORDS
                    .iter()
                    .map(|k| {
                        J::Obj(vec![("label".into(), jstr(k)), ("kind".into(), jnum(14))])
                    })
                    .collect();
                for s in crate::file_symbols(&text, &path) {
                    items.push(J::Obj(vec![
                        ("label".into(), jstr(&s.name)),
                        ("kind".into(), jnum(completion_kind(s.kind))),
                        ("detail".into(), jstr(&s.detail)),
                    ]));
                }
                send(&mut wout, id.unwrap_or(J::Null), &J::Obj(vec![("isIncomplete".into(), J::Bool(false)), ("items".into(), J::Arr(items))]));
            }
            other => {
                if wants_reply {
                    send_error(&mut wout, id.unwrap_or(J::Null), -32601, &format!("method not found: {}", other));
                }
                // unknown notifications are ignored (LSP $/ extensions, cancel, etc.)
            }
        }
    }
    if shutdown {
        0
    } else {
        1
    }
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_and_parse() {
        let v = jparse(r#"{"a":[1,2.5,"x\ny",true,null],"b":{"c":"é\\d"}}"#).expect("parses");
        assert_eq!(v.get("b").and_then(|b| b.get("c")).and_then(|c| c.as_str()), Some("é\\d"));
        let back = jtostr(&v);
        assert_eq!(jparse(&back).unwrap(), v);
    }

    #[test]
    fn lsp_range_extends_over_identifier() {
        let (sl, sc, el, ec) = lsp_range("let x = unknown_name\nzz", 1, 9);
        assert_eq!((sl, sc, el, ec), (0, 8, 0, 20));
    }

    #[test]
    fn uri_paths() {
        assert_eq!(uri_to_path("file:///home/u/main.strc"), "/home/u/main.strc");
        assert_eq!(uri_to_path("file:///C:/Users/h/main.strc"), "C:/Users/h/main.strc");
        assert_eq!(uri_to_path("file:///a/b%20c.strc"), "/a/b c.strc");
    }
}
