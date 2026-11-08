use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Toolchain {
    program: PathBuf,
    vcvars: Option<PathBuf>,
}

pub fn find_cc() -> Option<Toolchain> {
    for c in ["cc", "gcc", "clang"] {
        if which(c) {
            return Some(Toolchain { program: PathBuf::from(c), vcvars: None });
        }
    }
    if which("cl") {
        return Some(Toolchain { program: PathBuf::from("cl"), vcvars: None });
    }
    if cfg!(windows) {
        if let Some(t) = vs_toolchain() {
            return Some(t);
        }
    }
    None
}

fn which(name: &str) -> bool {
    if Path::new(name).exists() {
        return true;
    }
    let candidates: Vec<String> = if cfg!(windows) {
        vec![name.to_string(), format!("{}.exe", name)]
    } else {
        vec![name.to_string()]
    };
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for c in &candidates {
                let p = dir.join(c);
                if p.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(windows)]
fn vs_toolchain() -> Option<Toolchain> {
    let vswhere = [
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
        r"C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe",
    ]
    .iter()
    .find(|p| Path::new(p).exists())?;
    let out = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if root.is_empty() {
        return None;
    }
    let vcvars = Path::new(&root).join(r"VC\Auxiliary\Build\vcvars64.bat");
    if !vcvars.is_file() {
        return None;
    }
    let msvc_root = Path::new(&root).join(r"VC\Tools\MSVC");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&msvc_root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    versions.sort();
    let cl = versions.into_iter().rev().find_map(|v| {
        let cl = v.join(r"bin\Hostx64\x64\cl.exe");
        if cl.is_file() { Some(cl) } else { None }
    })?;
    Some(Toolchain { program: cl, vcvars: Some(vcvars) })
}

#[cfg(not(windows))]
fn vs_toolchain() -> Option<Toolchain> {
    None
}

fn is_cl(p: &Path) -> bool {
    p.file_name().map(|f| f == "cl" || f == "cl.exe").unwrap_or(false)
}

pub fn compile(c_file: &Path, rt_c: &Path, include_dir: &Path, out: &Path, release: bool) -> Result<(), String> {
    let tc = find_cc().ok_or_else(|| no_cc_error())?;
    let out_abs = if out.is_absolute() {
        out.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| format!("cannot resolve output path: {}", e))?.join(out)
    };
    if is_cl(&tc.program) {
        let out_res = if let Some(vcvars) = &tc.vcvars {
            let bat = include_dir.join("still_build.bat");
            let mut line = String::new();
            line.push_str(&format!("@cd /d \"{}\"\r\n", include_dir.display()));
            line.push_str(&format!("@call \"{}\" >nul 2>&1\r\n", vcvars.display()));
            line.push_str("@if errorlevel 1 exit /b 1\r\n");
            line.push_str(&format!("@\"{}\" /nologo /TC", tc.program.display()));
            line.push_str(&format!(" /I\"{}\"", include_dir.display()));
            line.push_str(&format!(" /Fe:\"{}\"", out_abs.display()));
            line.push_str(&format!(" \"{}\" \"{}\"", c_file.display(), rt_c.display()));
            if release {
                line.push_str(" /O2");
            } else {
                line.push_str(" /Od /Zi /DSTIL_DEBUG");
            }
            line.push_str("\r\n");
            std::fs::write(&bat, line).map_err(|e| format!("failed to write build script: {}", e))?;
            Command::new("cmd").args(["/C", &bat.display().to_string()]).output()
        } else {
            let mut cmd = Command::new(&tc.program);
            cmd.arg("/nologo")
                .arg("/TC")
                .arg(format!("/I{}", include_dir.display()))
                .arg(format!("/Fe:{}", out_abs.display()))
                .arg(c_file)
                .arg(rt_c);
            if release {
                cmd.arg("/O2");
            } else {
                cmd.arg("/Od").arg("/Zi").arg("/DSTIL_DEBUG");
            }
            cmd.current_dir(include_dir).output()
        };
        let out_res = out_res.map_err(|e| format!("failed to run cl: {}", e))?;
        if !out_res.status.success() {
            let stderr = String::from_utf8_lossy(&out_res.stderr);
            return Err(format!("C compilation failed:\n{}", stderr));
        }
        return Ok(());
    }
    let mut cmd = Command::new(&tc.program);
    cmd.arg("-std=gnu11")
        .arg(if release { "-O2" } else { "-O0" })
        .arg("-g")
        .arg(format!("-I{}", include_dir.display()))
        .arg("-D_FILE_OFFSET_BITS=64");
    if !release {
        cmd.arg("-DSTIL_DEBUG");
    }
    cmd.arg(c_file).arg(rt_c).arg("-o").arg(&out_abs);
    if !cfg!(windows) {
        cmd.arg("-lm").arg("-flto");
    }
    let out_res = cmd.output().map_err(|e| format!("failed to run {}: {}", tc.program.display(), e))?;
    if !out_res.status.success() {
        let stderr = String::from_utf8_lossy(&out_res.stderr);
        return Err(format!("C compilation failed:\n{}", stderr));
    }
    Ok(())
}

fn no_cc_error() -> String {
    if cfg!(windows) {
        "no C compiler found.\ninstall Visual Studio Build Tools with the 'Desktop development with C++' workload: https://visualstudio.microsoft.com/visual-cpp-build-tools/\nor put gcc (for example WinLibs) or clang on PATH".to_string()
    } else {
        "no C compiler found; install gcc or clang".to_string()
    }
}

pub fn tmp_build_dir(main: &Path) -> PathBuf {
    let mut d = std::env::temp_dir();
    let stem = main.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "still".into());
    let mut h = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher};
    main.hash(&mut h);
    d.push(format!("still_build_{}_{}", stem, h.finish()));
    let _ = std::fs::create_dir_all(&d);
    d
}
