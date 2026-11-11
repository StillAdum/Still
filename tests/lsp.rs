use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct Server {
    child: Child,
    rx: mpsc::Receiver<String>,
    si: std::process::ChildStdin,
}

fn frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

fn start_server() -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_still"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn still lsp");
    let si = child.stdin.take().unwrap();
    let so = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut r = BufReader::new(so);
        loop {
            let mut len: Option<usize> = None;
            let mut line = String::new();
            loop {
                line.clear();
                match r.read_line(&mut line) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
                let t = line.trim_end();
                if t.is_empty() {
                    break;
                }
                if let Some(v) = t.strip_prefix("Content-Length:") {
                    len = v.trim().parse().ok();
                }
            }
            let n = match len {
                Some(n) => n,
                None => return,
            };
            let mut buf = vec![0u8; n];
            if r.read_exact(&mut buf).is_err() {
                return;
            }
            if tx.send(String::from_utf8_lossy(&buf).to_string()).is_err() {
                return;
            }
        }
    });
    Server { child, rx, si }
}

impl Server {
    fn send(&mut self, body: &str) {
        self.si.write_all(frame(body).as_bytes()).expect("write to lsp server");
    }
    fn recv(&mut self, what: &str) -> String {
        match self.rx.recv_timeout(Duration::from_secs(60)) {
            Ok(s) => s,
            Err(_) => panic!("timed out waiting for {}", what),
        }
    }
}

#[test]
fn lsp_full_session() {
    let mut s = start_server();

    s.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    let init = s.recv("initialize response");
    assert!(init.contains("capabilities"), "init: {}", init);
    assert!(init.contains("documentSymbolProvider"), "init: {}", init);

    s.send(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#);

    // A document with a real checker error: `y` is never assigned.
    s.send(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/lsp_demo.strc","languageId":"still","version":1,"text":"x = 1\nfunc greet(who):\n    return who\nprint(greet(x))\nprint(y)\n"}}}"#,
    );
    let pub1 = s.recv("publishDiagnostics after didOpen");
    assert!(pub1.contains("publishDiagnostics"), "pub: {}", pub1);
    assert!(pub1.contains("diagnostics"), "pub: {}", pub1);
    assert!(pub1.contains("y"), "expected a diagnostic about y: {}", pub1);

    // documentSymbol sees the real symbols.
    s.send(
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///tmp/lsp_demo.strc"}}}"#,
    );
    let syms = s.recv("documentSymbol response");
    assert!(syms.contains("greet"), "symbols: {}", syms);
    assert!(syms.contains("\"kind\":13"), "variable kind 13: {}", syms);

    // Hover over the `greet` call: line index 2 = `print(greet(x))`, char 6 = 'g'.
    s.send(
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///tmp/lsp_demo.strc"},"position":{"line":2,"character":6}}}"#,
    );
    let hover = s.recv("hover response");
    assert!(hover.contains("greet"), "hover: {}", hover);
    assert!(hover.contains("who"), "hover should show the signature: {}", hover);

    // Completion contains keywords and file symbols.
    s.send(
        r#"{"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/lsp_demo.strc"},"position":{"line":0,"character":0}}}"#,
    );
    let comp = s.recv("completion response");
    assert!(comp.contains("func"), "keywords in completion: {}", comp);
    assert!(comp.contains("greet"), "file symbols in completion: {}", comp);

    // Fix the file via didChange (full sync): diagnostics should clear.
    s.send(
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///tmp/lsp_demo.strc","version":2},"contentChanges":[{"text":"x = 1\ny = 2\nprint(x + y)\n"}]}}"#,
    );
    let pub2 = s.recv("publishDiagnostics after fix");
    assert!(pub2.contains("publishDiagnostics"), "pub2: {}", pub2);
    assert!(pub2.contains("[]"), "diagnostics should be empty after fix: {}", pub2);

    // Unknown request gets a MethodNotFound error, server keeps going.
    s.send(r#"{"jsonrpc":"2.0","id":5,"method":"textDocument/whatever","params":{}}"#);
    let err = s.recv("method-not-found response");
    assert!(err.contains("-32601"), "err: {}", err);

    s.send(r#"{"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}"#);
    let sd = s.recv("shutdown response");
    assert!(sd.contains("\"result\":null"), "shutdown: {}", sd);
    s.send(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#);
    let status = s.child.wait().expect("wait after exit");
    assert!(status.success(), "exit code after shutdown should be 0");
}

#[test]
fn lsp_survives_garbage_and_bad_files() {
    let mut s = start_server();
    // Frames that are not valid JSON must not kill the server.
    s.send("not json at all");
    // Opening a doc whose `use` target is missing reports a load error.
    s.send(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/lsp_missing_dep.strc","languageId":"still","version":1,"text":"use does_not_exist_anywhere\nprint(1)\n"}}}"#,
    );
    let pub1 = s.recv("publishDiagnostics for missing use");
    assert!(pub1.contains("publishDiagnostics"), "pub: {}", pub1);
    assert!(pub1.contains("does_not_exist_anywhere"), "pub: {}", pub1);
    // Still alive and answering requests afterwards.
    s.send(
        r#"{"jsonrpc":"2.0","id":9,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/lsp_missing_dep.strc"},"position":{"line":0,"character":0}}}"#,
    );
    let comp = s.recv("completion after garbage");
    assert!(comp.contains("func"), "comp: {}", comp);
    s.send(r#"{"jsonrpc":"2.0","id":10,"method":"shutdown","params":null}"#);
    let _ = s.recv("shutdown response");
    s.send(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#);
    let status = s.child.wait().expect("wait after exit");
    assert!(status.success(), "clean exit");
}
