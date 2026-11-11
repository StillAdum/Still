use still::{build, compile_to_c, doctor, ide_check, ide_symbols, new_project, run, setup_cc, test_dir, VERSION};
use std::path::PathBuf;
use std::process::exit;

fn usage() {
    eprintln!("still {} - the Still programming language", VERSION);
    eprintln!();
    eprintln!("usage:");
    eprintln!("  still run FILE.strc [--release] [-- args...]");
    eprintln!("  still build FILE.strc -o OUT [--release] [--keep-c]");
    eprintln!("  still check FILE.strc");
    eprintln!("  still emit-c FILE.strc [-o OUT.c] [--release]");
    eprintln!("  still test DIR");
    eprintln!("  still new NAME");
    eprintln!("  still doctor");
    eprintln!("  still setup-cc");
    eprintln!("  still ide symbols FILE.strc");
    eprintln!("  still ide check FILE.strc");
    eprintln!("  still lsp");
    eprintln!("  still version");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        exit(2);
    }
    let cmd = args[0].as_str();
    let rest = &args[1..];
    match cmd {
        "version" | "--version" | "-V" => println!("still {}", VERSION),
        "run" => {
            let file = match rest.first() {
                Some(f) => PathBuf::from(f),
                None => {
                    eprintln!("still run needs a file");
                    exit(2);
                }
            };
            let mut release = false;
            let mut prog_args: Vec<String> = Vec::new();
            let mut i = 1;
            while i < rest.len() {
                if rest[i] == "--release" {
                    release = true;
                } else if rest[i] == "--" {
                    prog_args.extend(rest[i + 1..].iter().cloned());
                    break;
                } else if i > 1 {
                    prog_args.push(rest[i].clone());
                }
                i += 1;
            }
            match run(&file, release, &prog_args) {
                Ok(code) => exit(code),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        "build" => {
            let file = match rest.first() {
                Some(f) => PathBuf::from(f),
                None => {
                    eprintln!("still build needs a file");
                    exit(2);
                }
            };
            let mut out = file.file_stem().map(|s| PathBuf::from(s.to_string_lossy().to_string())).unwrap_or_else(|| PathBuf::from("out"));
            let mut release = false;
            let mut keep_c = false;
            let mut i = 1;
            while i < rest.len() {
                match rest[i].as_str() {
                    "-o" => {
                        if i + 1 < rest.len() {
                            out = PathBuf::from(&rest[i + 1]);
                            i += 1;
                        }
                    }
                    "--release" => release = true,
                    "--keep-c" => keep_c = true,
                    _ => {}
                }
                i += 1;
            }
            match build(&file, &out, release, keep_c) {
                Ok(p) => println!("built {}", p.display()),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        "check" => {
            let file = match rest.first() {
                Some(f) => PathBuf::from(f),
                None => {
                    eprintln!("still check needs a file");
                    exit(2);
                }
            };
            match compile_to_c(&file, false) {
                Ok(o) => {
                    if o.diags.is_empty() {
                        println!("ok");
                    } else {
                        for d in &o.diags {
                            eprintln!("{}", d);
                        }
                        exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        "emit-c" => {
            let file = match rest.first() {
                Some(f) => PathBuf::from(f),
                None => {
                    eprintln!("still emit-c needs a file");
                    exit(2);
                }
            };
            let mut out = file.with_extension("c");
            let mut release = false;
            let mut i = 1;
            while i < rest.len() {
                match rest[i].as_str() {
                    "-o" => {
                        if i + 1 < rest.len() {
                            out = PathBuf::from(&rest[i + 1]);
                            i += 1;
                        }
                    }
                    "--release" => release = true,
                    _ => {}
                }
                i += 1;
            }
            match compile_to_c(&file, release) {
                Ok(o) => {
                    if o.diags.is_empty() {
                        if let Err(e) = std::fs::write(&out, o.c) {
                            eprintln!("cannot write {}: {}", out.display(), e);
                            exit(1);
                        }
                        println!("wrote {}", out.display());
                    } else {
                        for d in &o.diags {
                            eprintln!("{}", d);
                        }
                        exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        "test" => {
            let dir = match rest.first() {
                Some(d) => PathBuf::from(d),
                None => PathBuf::from("."),
            };
            match test_dir(&dir, false) {
                Ok(results) => {
                    let mut failed = 0;
                    for r in &results {
                        if r.passed {
                            println!("ok      {}", r.name);
                        } else {
                            failed += 1;
                            println!("FAIL    {}", r.name);
                            if !r.detail.is_empty() {
                                for line in r.detail.lines().take(12) {
                                    println!("        {}", line);
                                }
                            }
                        }
                    }
                    println!("{} passed, {} failed", results.len() - failed, failed);
                    if failed > 0 {
                        exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        "new" => {
            let name = match rest.first() {
                Some(n) => n.clone(),
                None => {
                    eprintln!("still new needs a project name");
                    exit(2);
                }
            };
            match new_project(&name) {
                Ok(d) => println!("created {} (edit {}/src/main.strc, then: still run {}/src/main.strc)", d.display(), name, name),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        "ide" => {
            if rest.len() < 2 {
                eprintln!("still ide needs a command and a file");
                exit(2);
            }
            let file = PathBuf::from(&rest[1]);
            match rest[0].as_str() {
                "symbols" => match ide_symbols(&file) {
                    Ok(s) => println!("{}", s),
                    Err(e) => {
                        eprintln!("{}", e);
                        exit(1);
                    }
                },
                "check" => match ide_check(&file) {
                    Ok(s) => println!("{}", s),
                    Err(e) => {
                        eprintln!("{}", e);
                        exit(1);
                    }
                },
                _ => {
                    eprintln!("unknown ide command '{}'", rest[0]);
                    exit(2);
                }
            }
        }
        "doctor" => {
            println!("{}", doctor());
        }
        "lsp" => {
            let code = still::lsp::serve();
            exit(code);
        }
        "setup-cc" => {
            match setup_cc() {
                Ok(p) => println!("ready: C compiler located at {}", p.display()),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        _ => {
            usage();
            exit(2);
        }
    }
}
