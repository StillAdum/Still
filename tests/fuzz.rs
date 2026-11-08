use still::lexer::{Lexer, Tok};
use still::parser;

fn seed_rng(seed: u64) -> u64 {
    let mut x = seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

fn garbage(seed: u64, len: usize) -> String {
    let alphabet: Vec<char> = "abcdefgXYZ_0123456789 \n\t\"':=+-*/%()[]{}<>,.#@!?\\\n    func type if else while for in return break continue pass use extern none true false".chars().collect();
    let mut s = seed;
    let mut out = String::new();
    for _ in 0..len {
        s = seed_rng(s) | 1;
        let idx = (s % alphabet.len() as u64) as usize;
        out.push(alphabet[idx]);
    }
    out
}

#[test]
fn lexer_never_panics() {
    for i in 0..3000u64 {
        let src = garbage(i, 120);
        let lex = Lexer::new(&src, "fuzz.strc");
        let (_, _) = lex.run();
    }
}

#[test]
fn parser_never_panics() {
    let mut panics = 0;
    for i in 0..3000u64 {
        let src = garbage(i + 77777, 140);
        let result = std::panic::catch_unwind(|| {
            let _ = parser::parse(&src, "fuzz.strc");
        });
        if result.is_err() {
            panics += 1;
        }
    }
    assert_eq!(panics, 0, "parser panicked on garbage input");
}

#[test]
fn parser_handles_deep_nesting() {
    let handle = std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        for depth in [10, 50, 120, 2000] {
            let src = format!("x = {}1{}\n", "(".repeat(depth), ")".repeat(depth));
            let (stmts, diags) = parser::parse(&src, "deep.strc");
            let _ = (stmts, diags);
        }
    }).unwrap();
    handle.join().unwrap();
}

#[test]
fn tokens_are_finite() {
    let src = garbage(42, 5000);
    let lex = Lexer::new(&src, "f.strc");
    let (toks, _) = lex.run();
    assert!(toks.len() < 20000, "token explosion: {}", toks.len());
    assert!(matches!(toks.last().unwrap().tok, Tok::Eof));
}
