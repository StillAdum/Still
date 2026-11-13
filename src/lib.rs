pub mod ast;
pub mod cc;
pub mod check;
pub mod emit_c;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod span;

pub use cc::{doctor, setup_cc};

use check::{check_program, ModSrc};
use span::Diag;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const VERSION: &str = "0.4.0";

pub const STD_MATH: &str = include_str!("stdlib/math.strc");
pub const STD_SORT: &str = include_str!("stdlib/sort.strc");
pub const STD_RANDOM: &str = include_str!("stdlib/random.strc");
pub const STD_TIME: &str = include_str!("stdlib/time.strc");
pub const STD_IO: &str = include_str!("stdlib/io.strc");
pub const STD_OS: &str = include_str!("stdlib/os.strc");
pub const STD_VEC: &str = include_str!("stdlib/vec.strc");
pub const STD_GUI: &str = include_str!("stdlib/gui.strc");
pub const STD_NET: &str = include_str!("stdlib/net.strc");
pub const STD_NET_ROLLBACK: &str = include_str!("stdlib/net_rollback.strc");
pub const STD_AUDIO: &str = include_str!("stdlib/audio.strc");
pub const STD_PHYSICS: &str = include_str!("stdlib/physics.strc");
pub const STD_PATHFINDING: &str = include_str!("stdlib/pathfinding.strc");
pub const STD_WEB: &str = include_str!("stdlib/web.strc");
pub const STD_REFLECT: &str = include_str!("stdlib/reflect.strc");
pub const STD_SERIALIZE: &str = include_str!("stdlib/serialize.strc");
pub const STD_DSL: &str = include_str!("stdlib/dsl.strc");
pub const STD_UNITS: &str = include_str!("stdlib/units.strc");
pub const STD_FIXED: &str = include_str!("stdlib/fixed.strc");
pub const STD_UNDO: &str = include_str!("stdlib/undo.strc");
pub const STD_REACTIVE: &str = include_str!("stdlib/reactive.strc");
pub const STD_SPATIAL: &str = include_str!("stdlib/spatial.strc");
pub const STD_ARENA: &str = include_str!("stdlib/arena.strc");
pub const STD_TEXT: &str = include_str!("stdlib/text.strc");
pub const STD_PARSER_KIT: &str = include_str!("stdlib/parser_kit.strc");
pub const STD_WIN32: &str = include_str!("stdlib/win32.strc");
pub const STD_DIRECTX: &str = include_str!("stdlib/directx.strc");
pub const STD_MEDIA: &str = include_str!("stdlib/media.strc");
pub const STD_IPC: &str = include_str!("stdlib/ipc.strc");
pub const STD_DEBUG: &str = include_str!("stdlib/debug.strc");
pub const STD_HOTRELOAD: &str = include_str!("stdlib/hotreload.strc");
pub const STD_SANDBOX: &str = include_str!("stdlib/sandbox.strc");
pub const STD_ANIMATION: &str = include_str!("stdlib/animation.strc");
pub const STD_PARTICLES: &str = include_str!("stdlib/particles.strc");
pub const STD_PROCGEN: &str = include_str!("stdlib/procgen.strc");
pub const STD_GAME_SYSTEMS: &str = include_str!("stdlib/game_systems.strc");
pub const STD_AI: &str = include_str!("stdlib/ai.strc");
pub const STD_I18N: &str = include_str!("stdlib/i18n.strc");
pub const STD_SAVE: &str = include_str!("stdlib/save.strc");
pub const STD_INPUT: &str = include_str!("stdlib/input.strc");
pub const STD_CAMERA: &str = include_str!("stdlib/camera.strc");
pub const STD_REPLAY: &str = include_str!("stdlib/replay.strc");
pub const STD_PLATFORM: &str = include_str!("stdlib/platform.strc");
pub const STD_MODDING: &str = include_str!("stdlib/modding.strc");
pub const STD_SERVER: &str = include_str!("stdlib/server.strc");
pub const STD_ANALYTICS: &str = include_str!("stdlib/analytics.strc");
pub const STD_ANTIDDOS: &str = include_str!("stdlib/antiddos.strc");
pub const STD_PY: &str = include_str!("stdlib/py.strc");

pub fn std_modules() -> Vec<(&'static str, &'static str)> {
    vec![
        ("math", STD_MATH),
        ("sort", STD_SORT),
        ("random", STD_RANDOM),
        ("time", STD_TIME),
        ("io", STD_IO),
        ("os", STD_OS),
        ("vec", STD_VEC),
        ("gui", STD_GUI),
        ("net", STD_NET),
        ("net_rollback", STD_NET_ROLLBACK),
        ("audio", STD_AUDIO),
        ("physics", STD_PHYSICS),
        ("pathfinding", STD_PATHFINDING),
        ("web", STD_WEB),
        ("reflect", STD_REFLECT),
        ("serialize", STD_SERIALIZE),
        ("dsl", STD_DSL),
        ("units", STD_UNITS),
        ("fixed", STD_FIXED),
        ("undo", STD_UNDO),
        ("reactive", STD_REACTIVE),
        ("spatial", STD_SPATIAL),
        ("arena", STD_ARENA),
        ("text", STD_TEXT),
        ("parser_kit", STD_PARSER_KIT),
        ("win32", STD_WIN32),
        ("directx", STD_DIRECTX),
        ("media", STD_MEDIA),
        ("ipc", STD_IPC),
        ("debug", STD_DEBUG),
        ("hotreload", STD_HOTRELOAD),
        ("sandbox", STD_SANDBOX),
        ("animation", STD_ANIMATION),
        ("particles", STD_PARTICLES),
        ("procgen", STD_PROCGEN),
        ("game_systems", STD_GAME_SYSTEMS),
        ("ai", STD_AI),
        ("i18n", STD_I18N),
        ("save", STD_SAVE),
        ("input", STD_INPUT),
        ("camera", STD_CAMERA),
        ("replay", STD_REPLAY),
        ("platform", STD_PLATFORM),
        ("modding", STD_MODDING),
        ("server", STD_SERVER),
        ("analytics", STD_ANALYTICS),
        ("antiddos", STD_ANTIDDOS),
        ("py", STD_PY),
    ]
}

pub struct Loaded {
    pub sources: Vec<ModSrc>,
    pub externs_needed: bool,
}

fn load_module(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map(|s| s.strip_prefix('\u{feff}').unwrap_or(&s).replace("\r\n", "\n"))
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))
}

fn normalize_src(s: &str) -> String {
    s.strip_prefix('\u{feff}').unwrap_or(s).replace("\r\n", "\n")
}

/// Scan a module's statements for `use` and enqueue the dependency sources
/// (embedded std modules first, then local .strc files next to the main file).
fn collect_uses(
    stmts: &[ast::Stmt],
    main_dir: &Path,
    std: &HashMap<String, &'static str>,
    loaded: &HashSet<String>,
    queue: &mut Vec<(String, String, Option<PathBuf>)>,
) -> Result<(), String> {
    for s in stmts {
        if let ast::Stmt::Use { target, .. } = s {
            if target.ends_with(".strc") {
                let p = main_dir.join(target);
                let stem = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| "m".into());
                if !loaded.contains(&stem) && !queue.iter().any(|(n, _, _)| *n == stem) {
                    let text = load_module(&p).map_err(|e| format!("use '{}' failed: {}", target, e))?;
                    queue.push((stem, text, Some(p)));
                }
            } else if let Some(text) = std.get(target) {
                if !loaded.contains(target) && !queue.iter().any(|(n, _, _)| n == target) {
                    queue.push((target.clone(), text.to_string(), None));
                }
            } else {
                let local = main_dir.join(format!("{}.strc", target));
                if local.exists() && !loaded.contains(target) && !queue.iter().any(|(n, _, _)| n == target) {
                    let text = load_module(&local).map_err(|e| format!("use '{}' failed: {}", target, e))?;
                    queue.push((target.clone(), text, Some(local)));
                }
            }
        }
    }
    Ok(())
}

/// Load a program whose main module is ALREADY parsed. Shared by
/// load_program (from disk) and check_text (editor text).
fn load_parsed(
    main_name: &str,
    main_stmts: Vec<ast::Stmt>,
    main_dir: &Path,
) -> Result<Loaded, String> {
    let std: HashMap<String, &'static str> = std_modules().into_iter().map(|(n, s)| (n.to_string(), s)).collect();
    let mut sources: Vec<ModSrc> = Vec::new();
    let mut loaded: HashSet<String> = HashSet::new();
    let mut queue: Vec<(String, String, Option<PathBuf>)> = Vec::new();
    loaded.insert(main_name.to_string());
    collect_uses(&main_stmts, main_dir, &std, &loaded, &mut queue)?;
    sources.push(ModSrc { name: main_name.to_string(), stmts: main_stmts });
    while let Some((name, src, path)) = queue.pop() {
        if loaded.contains(&name) {
            continue;
        }
        loaded.insert(name.clone());
        let (stmts, diags) = parser::parse(&src, path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| name.clone()).as_str());
        if !diags.is_empty() {
            let msgs: Vec<String> = diags.iter().map(|d| d.to_string()).collect();
            return Err(format!("{}\n{}", name, msgs.join("\n")));
        }
        collect_uses(&stmts, main_dir, &std, &loaded, &mut queue)?;
        sources.push(ModSrc { name: name.clone(), stmts });
    }
    Ok(Loaded { sources, externs_needed: true })
}

pub fn load_program(main: &Path) -> Result<Loaded, String> {
    let src = load_module(main)?;
    let norm = normalize_src(&src);
    let label = main.to_string_lossy().to_string();
    let (stmts, diags) = parser::parse(&norm, label.as_str());
    if !diags.is_empty() {
        let msgs: Vec<String> = diags.iter().map(|d| d.to_string()).collect();
        return Err(format!("main\n{}", msgs.join("\n")));
    }
    let main_dir = main.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
    load_parsed("main", stmts, &main_dir)
}

/// Check a program from in-memory text (LSP didOpen/didChange). `main` is the
/// file the text belongs to on disk — `use` statements resolve relative to its
/// directory, so sibling modules still load while the buffer is unsaved.
pub fn check_text(src: &str, main: &Path) -> Result<CompileOutput, String> {
    let norm = normalize_src(src);
    let label = main.to_string_lossy().to_string();
    let (stmts, pdiags) = parser::parse(&norm, label.as_str());
    if !pdiags.is_empty() {
        return Ok(CompileOutput { c: String::new(), diags: pdiags });
    }
    let main_dir = main.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
    let loaded = load_parsed("main", stmts, &main_dir)?;
    let (prog, diags) = check_program(loaded.sources, "main", true);
    if !diags.is_empty() {
        return Ok(CompileOutput { c: String::new(), diags });
    }
    Ok(CompileOutput { c: emit_c::emit(&prog), diags: Vec::new() })
}

pub struct CompileOutput {
    pub c: String,
    pub diags: Vec<Diag>,
}

pub fn compile_to_c(main: &Path, release: bool) -> Result<CompileOutput, String> {
    let loaded = load_program(main)?;
    let (prog, diags) = check_program(loaded.sources, "main", !release);
    if !diags.is_empty() {
        return Ok(CompileOutput { c: String::new(), diags });
    }
    let c = emit_c::emit(&prog);
    Ok(CompileOutput { c, diags: Vec::new() })
}

pub fn write_runtime(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("still_rt.h"), include_str!("rt/still_rt.h")).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("still_rt.c"), include_str!("rt/still_rt.c")).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn build(main: &Path, out: &Path, release: bool, keep_c: bool) -> Result<PathBuf, String> {
    let out_c = compile_to_c(main, release)?;
    if !out_c.diags.is_empty() {
        return Err(out_c.diags.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n"));
    }
    let dir = cc::tmp_build_dir(main);
    write_runtime(&dir)?;
    let cfile = dir.join("gen.c");
    std::fs::write(&cfile, &out_c.c).map_err(|e| e.to_string())?;
    let exe = out.to_path_buf();
    cc::compile(&cfile, &dir.join("still_rt.c"), &dir, &exe, release)?;
    if keep_c {
        let mut kept = main.file_stem().map(|s| PathBuf::from(s.to_string_lossy().to_string() + ".c")).unwrap_or_else(|| PathBuf::from("out.c"));
        if let Some(parent) = main.parent() {
            kept = parent.join(&kept);
        }
        let _ = std::fs::copy(&cfile, &kept);
    }
    Ok(exe)
}

pub fn run(main: &Path, release: bool, args: &[String]) -> Result<i32, String> {
    let dir = cc::tmp_build_dir(main);
    let exe = dir.join("still_out");
    build(main, &exe, release, false)?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(args);
    let status = cmd.status().map_err(|e| format!("failed to run: {}", e))?;
    Ok(status.code().unwrap_or(1))
}

pub fn new_project(name: &str) -> Result<PathBuf, String> {
    let dir = PathBuf::from(name);
    if dir.exists() {
        return Err(format!("'{}' already exists", name));
    }
    let src = dir.join("src");
    let tests = dir.join("tests");
    std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&tests).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir.join("lib")).map_err(|e| e.to_string())?;
    std::fs::write(
        src.join("main.strc"),
        "# welcome to Still\n\nname = \"world\"\nprint(\"hello, {name}!\")\n\nnums = [1, 2, 3]\ntotal = 0\nfor n in nums:\n    total += n\n\nprint(\"sum is {total}\")\n",
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(tests.join("main_test.strc"), "check(1 + 1 == 2, \"math works\")\nprint(\"ok\")\n").map_err(|e| e.to_string())?;
    Ok(dir)
}

pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub fn test_dir(dir: &Path, release: bool) -> Result<Vec<TestResult>, String> {
    let mut results = Vec::new();
    let valid = dir.join("valid");
    let invalid = dir.join("invalid");
    let bdir = cc::tmp_build_dir(dir);
    write_runtime(&bdir)?;
    let mut entries: Vec<PathBuf> = Vec::new();
    if valid.exists() {
        for e in std::fs::read_dir(&valid).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?.path();
            if e.extension().map(|x| x == "strc").unwrap_or(false) {
                entries.push(e);
            }
        }
    }
    entries.sort();
    for f in entries {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let expected_path = f.with_extension("expected");
        let expected = std::fs::read_to_string(&expected_path).unwrap_or_default().replace("\r\n", "\n");
        match compile_and_capture(&f, &bdir, release) {
            Ok((stdout, code)) => {
                let ok = code == 0 && stdout.replace("\r\n", "\n") == expected;
                results.push(TestResult {
                    name,
                    passed: ok,
                    detail: if ok { String::new() } else { format!("exit {} stdout:\n{}\nexpected:\n{}", code, stdout, expected) },
                });
            }
            Err(e) => {
                results.push(TestResult { name, passed: false, detail: format!("compile error: {}", e) });
            }
        }
    }
    let mut ientries: Vec<PathBuf> = Vec::new();
    if invalid.exists() {
        for e in std::fs::read_dir(&invalid).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?.path();
            if e.extension().map(|x| x == "strc").unwrap_or(false) {
                ientries.push(e);
            }
        }
    }
    ientries.sort();
    for f in ientries {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let expected_path = f.with_extension("expected");
        let expected = std::fs::read_to_string(&expected_path).unwrap_or_default().replace("\r\n", "\n");
        let src = std::fs::read_to_string(&f).map_err(|e| e.to_string())?.replace("\r\n", "\n");
        let (stmts, diags) = parser::parse(&src, f.to_string_lossy().to_string().as_str());
        let mut msgs: Vec<String> = diags.iter().map(|d| d.to_string()).collect();
        if msgs.is_empty() && !stmts.is_empty() {
            match compile_to_c(&f, true) {
                Ok(o) => {
                    if !o.diags.is_empty() {
                        msgs = o.diags.iter().map(|d| d.to_string()).collect();
                    }
                }
                Err(e) => msgs.push(e),
            }
        }
        let joined = msgs.join("\n");
        let ok = !joined.is_empty() && (expected.trim().is_empty() || joined.contains(expected.trim()));
        results.push(TestResult {
            name,
            passed: ok,
            detail: if ok { String::new() } else { format!("got:\n{}", joined) },
        });
    }
    Ok(results)
}

fn compile_and_capture(f: &Path, bdir: &Path, release: bool) -> Result<(String, i32), String> {
    let out = compile_to_c(f, release)?;
    if !out.diags.is_empty() {
        return Err(out.diags.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n"));
    }
    let cfile = bdir.join(format!("t_{}.c", f.file_stem().unwrap().to_string_lossy()));
    std::fs::write(&cfile, &out.c).map_err(|e| e.to_string())?;
    let exe = bdir.join(format!("t_{}", f.file_stem().unwrap().to_string_lossy()));
    cc::compile(&cfile, &bdir.join("still_rt.c"), bdir, &exe, release)?;
    let outp = std::process::Command::new(&exe).current_dir(bdir).output().map_err(|e| format!("run failed: {}", e))?;
    Ok((String::from_utf8_lossy(&outp.stdout).to_string(), outp.status.code().unwrap_or(1)))
}

pub struct Sym {
    pub name: String,
    pub kind: &'static str,
    pub line: u32,
    pub detail: String,
}

/// Top-level symbols of one module, shared by the VSCode JSON commands and the
/// LSP (documentSymbol/hover/completion).
pub fn file_symbols(src: &str, label: &str) -> Vec<Sym> {
    let norm = normalize_src(src);
    let (stmts, _) = parser::parse(&norm, label);
    let mut syms: Vec<Sym> = Vec::new();
    for s in &stmts {
        match s {
            ast::Stmt::FuncDef(f) => {
                let params: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
                syms.push(Sym {
                    name: f.name.clone(),
                    kind: "function",
                    line: f.span.line,
                    detail: params.join(", "),
                });
            }
            ast::Stmt::TypeDef(t) => {
                syms.push(Sym { name: t.name.clone(), kind: "type", line: t.span.line, detail: "type".into() });
                for m in &t.methods {
                    syms.push(Sym {
                        name: m.name.clone(),
                        kind: "method",
                        line: m.span.line,
                        detail: format!("{}.{}", t.name, m.name),
                    });
                }
                for fl in &t.fields {
                    syms.push(Sym {
                        name: fl.name.clone(),
                        kind: "field",
                        line: fl.span.line,
                        detail: format!("{}.{}", t.name, fl.name),
                    });
                }
            }
            ast::Stmt::Use { target, alias, .. } => {
                syms.push(Sym {
                    name: alias.clone().unwrap_or_else(|| target.clone()),
                    kind: "module",
                    line: 0,
                    detail: format!("use {}", target),
                });
            }
            ast::Stmt::Assign { targets, op: None, .. } => {
                for t in targets {
                    if let ast::Expr::Ident(n, sp) = t {
                        syms.push(Sym { name: n.clone(), kind: "variable", line: sp.line, detail: String::new() });
                    }
                }
            }
            _ => {}
        }
    }
    syms
}

pub fn ide_symbols(main: &Path) -> Result<String, String> {
    let src = load_module(main)?;
    let label = main.to_string_lossy().to_string();
    let (_, diags) = parser::parse(normalize_src(&src).as_str(), label.as_str());
    if !diags.is_empty() {
        return Ok(format!("{{\"symbols\":[],\"errors\":{}}}", diags.len()));
    }
    let syms = file_symbols(&src, label.as_str());
    let out: Vec<String> = syms
        .iter()
        .map(|s| {
            format!(
                "{{\"name\":\"{}\",\"kind\":\"{}\",\"line\":{},\"detail\":\"{}\"}}",
                s.name, s.kind, s.line, s.detail
            )
        })
        .collect();
    Ok(format!("{{\"symbols\":[{}]}}", out.join(",")))
}

pub fn ide_check(main: &Path) -> Result<String, String> {
    let diags_json = |diags: &[Diag]| -> String {
        let items: Vec<String> = diags
            .iter()
            .map(|d| {
                format!(
                    "{{\"file\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\"}}",
                    d.span.file.replace('\\', "/").replace('"', "\\\""),
                    d.span.line,
                    d.span.col,
                    d.msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    };
    let src = load_module(main)?;
    let (_, pdiags) = parser::parse(&src, main.to_string_lossy().to_string().as_str());
    if !pdiags.is_empty() {
        return Ok(diags_json(&pdiags));
    }
    match compile_to_c(main, false) {
        Ok(o) => Ok(diags_json(&o.diags)),
        Err(e) => {
            let sp = span::Span::new(std::rc::Rc::from(main.to_string_lossy().to_string().as_str()), 1, 1);
            let d = span::err(&sp, e.replace('"', "\\\"").replace('\n', " "));
            Ok(diags_json(&[d]))
        }
    }
}
