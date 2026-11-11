//! Python bridge integration test.
//!
//! Runs a real program through `still run` that embeds CPython. Skips
//! gracefully (passes) when libpython is not present in the environment, so
//! CI machines without Python stay green.

use std::process::Command;

#[test]
fn python_bridge_runs() {
    let dir = std::env::temp_dir().join("still_py_bridge_test");
    let _ = std::fs::create_dir_all(&dir);
    let prog = dir.join("py_bridge.strc");
    std::fs::write(
        &prog,
        r#"
use py

func demo():
    if py.init() != 0:
        print("SKIP")
        return
    py.run("import math")
    py.run("n = 6 * 7")
    print(py.eval("math.sqrt(2)"))
    print(py.eval("n"))
    x = 21.0
    print(py.eval("{x} * 2"))

demo()
"#,
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_still"))
        .arg("run")
        .arg(&prog)
        .output()
        .expect("run python bridge program");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if stdout.trim() == "SKIP" {
        eprintln!("libpython not available in this environment - skipping python_bridge test");
        return;
    }
    assert!(
        out.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        stdout,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("1.41421356"), "sqrt(2) via eval: {}", stdout);
    assert!(stdout.contains("42"), "state shared between run and eval: {}", stdout);
    assert!(stdout.contains("42.0"), "interpolated float arg: {}", stdout);
}
