use std::fmt;

#[derive(Clone, Debug)]
pub struct Span {
    pub file: std::rc::Rc<str>,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(file: std::rc::Rc<str>, line: u32, col: u32) -> Span {
        Span { file, line, col }
    }
}

#[derive(Clone, Debug)]
pub struct Diag {
    pub span: Span,
    pub msg: String,
}

pub fn err(span: &Span, msg: String) -> Diag {
    Diag { span: span.clone(), msg }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}:{}: error: {}", self.span.file, self.span.line, self.span.col, self.msg)
    }
}

pub fn report(diags: &[Diag], limit: usize) {
    for d in diags.iter().take(limit) {
        eprintln!("{}", d);
    }
    if diags.len() > limit {
        eprintln!("... and {} more errors", diags.len() - limit);
    }
}

pub fn did_you_mean(name: &str, options: &[String]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for o in options {
        let d = levenshtein(name, o);
        if d <= 2 && (best.is_none() || d < best.as_ref().unwrap().0) {
            best = Some((d, o.clone()));
        }
    }
    best.map(|(_, s)| s)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
