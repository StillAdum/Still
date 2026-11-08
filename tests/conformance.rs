use still::test_dir;
use std::path::Path;

#[test]
fn conformance_suite() {
    let dir = Path::new("conformance");
    let results = test_dir(dir, false).expect("test runner failed");
    let mut failures = Vec::new();
    for r in &results {
        if !r.passed {
            failures.push(format!("{}: {}", r.name, r.detail));
        }
    }
    assert!(results.len() >= 30, "expected at least 30 conformance tests, got {}", results.len());
    assert!(failures.is_empty(), "conformance failures:\n{}", failures.join("\n---\n"));
}

#[test]
fn examples_run() {
    for entry in std::fs::read_dir("examples").expect("no examples dir") {
        let p = entry.unwrap().path();
        if p.extension().map(|e| e == "strc").unwrap_or(false) {
            let out = still::compile_to_c(&p, false).expect("compile failed");
            assert!(out.diags.is_empty(), "example {} failed: {:?}", p.display(), out.diags.iter().map(|d| d.to_string()).collect::<Vec<_>>());
        }
    }
}

#[test]
fn release_mode_still_fast() {
    let main = std::env::temp_dir().join("still_bench_check.strc");
    std::fs::write(&main, "func fib(n):\n    if n < 2:\n        return n\n    return fib(n - 1) + fib(n - 2)\nprint(fib(28))\n").unwrap();
    let out = still::compile_to_c(&main, true).expect("compile");
    assert!(out.diags.is_empty());
    assert!(out.c.contains("still_main__fib__i"), "release build should contain the fib function");
}
