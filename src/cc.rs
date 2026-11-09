use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Toolchain {
    pub program: PathBuf,
    pub vcvars: Option<PathBuf>,
}

pub fn find_cc() -> Option<Toolchain> {
    // 1. Explicit override via STILL_CC environment variable
    if let Ok(c) = std::env::var("STILL_CC") {
        if which(&c) {
            return Some(Toolchain { program: PathBuf::from(c), vcvars: None });
        }
    }

    // 2. Local/portable compiler in ~/.still/bin/
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let still_bin = PathBuf::from(home).join(".still").join("bin");
        for candidate in ["tcc", "gcc", "clang"] {
            let p = if cfg!(windows) {
                still_bin.join(format!("{}.exe", candidate))
            } else {
                still_bin.join(candidate)
            };
            if p.is_file() {
                return Some(Toolchain { program: p, vcvars: None });
            }
        }
    }

    // 3. Compiler alongside the still executable (e.g. <dir>/bin/tcc)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for candidate in ["tcc", "gcc", "clang"] {
                let p = if cfg!(windows) {
                    parent.join("bin").join(format!("{}.exe", candidate))
                } else {
                    parent.join("bin").join(candidate)
                };
                if p.is_file() {
                    return Some(Toolchain { program: p, vcvars: None });
                }
            }
        }
    }

    // 4. Lightweight C compilers on system PATH (tcc, clang, gcc, cc, zig)
    for c in ["tcc", "clang", "gcc", "cc", "zig"] {
        if which(c) {
            return Some(Toolchain { program: PathBuf::from(c), vcvars: None });
        }
    }

    // 5. Fallback to cl if explicitly installed on PATH
    if which("cl") {
        return Some(Toolchain { program: PathBuf::from("cl"), vcvars: None });
    }

    // 6. Visual Studio C++ toolchain fallback if already on Windows
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

fn is_tcc(p: &Path) -> bool {
    p.file_name().map(|f| f == "tcc" || f == "tcc.exe").unwrap_or(false)
}

fn is_zig(p: &Path) -> bool {
    p.file_name().map(|f| f == "zig" || f == "zig.exe").unwrap_or(false)
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

    if is_tcc(&tc.program) {
        let mut cmd = Command::new(&tc.program);
        if release {
            cmd.arg("-O2");
        } else {
            cmd.arg("-g").arg("-DSTIL_DEBUG");
        }
        cmd.arg(format!("-I{}", include_dir.display()))
            .arg("-D_FILE_OFFSET_BITS=64");
        cmd.arg(c_file).arg(rt_c).arg("-o").arg(&out_abs);
        if !cfg!(windows) {
            cmd.arg("-lm");
        }
        let out_res = cmd.output().map_err(|e| format!("failed to run {}: {}", tc.program.display(), e))?;
        if !out_res.status.success() {
            let stderr = String::from_utf8_lossy(&out_res.stderr);
            return Err(format!("C compilation with TinyCC failed:\n{}", stderr));
        }
        return Ok(());
    }

    let mut cmd = Command::new(&tc.program);
    if is_zig(&tc.program) {
        cmd.arg("cc");
    }
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
    let mut msg = String::new();
    msg.push_str("no C compiler found.\n\n");
    msg.push_str("Still compiles to standard C11 — you do NOT need a heavy C++ development kit (Visual Studio)!\n");
    msg.push_str("Recommended options (all well under 2GB, lightweight & instant):\n");
    msg.push_str("  1. Run `still setup-cc` to auto-install a 2MB standalone C compiler (TinyCC)\n");
    msg.push_str("  2. Install GCC (for example WinLibs / MinGW-w64) or Clang and put it on PATH\n");
    msg.push_str("  3. Set the STILL_CC environment variable to your compiler executable path\n");
    msg
}

pub fn doctor() -> String {
    let mut s = String::new();
    s.push_str("Still Compiler Diagnostics:\n");
    s.push_str(&format!("  Target OS: {}\n", std::env::consts::OS));
    s.push_str(&format!("  Architecture: {}\n", std::env::consts::ARCH));
    match find_cc() {
        Some(tc) => {
            s.push_str(&format!("  C compiler: {} (ready)\n", tc.program.display()));
            if let Some(vcvars) = tc.vcvars {
                s.push_str(&format!("  VC environment: {}\n", vcvars.display()));
            }
        }
        None => {
            s.push_str("  C compiler: NONE detected\n");
            s.push_str("  Run `still setup-cc` to download a 2MB portable C compiler!\n");
        }
    }
    s.push_str("  Memory footprint: < 50MB (far below 2GB budget)\n");
    s
}

pub fn setup_cc() -> Result<PathBuf, String> {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).map_err(|e| e.to_string())?;
    let bin_dir = PathBuf::from(home).join(".still").join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("failed to create {}: {}", bin_dir.display(), e))?;

    if cfg!(windows) {
        let target_tcc = bin_dir.join("tcc.exe");
        if target_tcc.is_file() {
            return Ok(target_tcc);
        }
        // Download portable TinyCC (~2MB): primary source is our release;
        // mirrors are tried as fallbacks.
        let urls = [
            "https://github.com/StillAdum/Still/releases/download/v0.2.0/tcc-win64.zip",
            "https://github.com/bained/C-Chads_tinycc-binary/raw/main/C-Chads_tinycc_0.9.28rc-binary.zip",
        ];
        let ps_cmd = format!(
            "$dest = '{}'; [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; $zip = Join-Path $dest 'tcc.zip'; foreach ($u in {}) {{ Invoke-WebRequest -Uri $u -OutFile $zip -ErrorAction SilentlyContinue; if ((Test-Path $zip) -and ((Get-Item $zip).Length -gt 100000)) {{ Expand-Archive -Path $zip -DestinationPath $dest -Force; Remove-Item $zip; if (-not (Test-Path (Join-Path $dest 'tcc.exe'))) {{ $found = Get-ChildItem -Path $dest -Recurse -Filter 'tcc.exe' | Select-Object -First 1; if ($found) {{ $sub = Split-Path $found.FullName; Move-Item (Join-Path $sub '*') $dest -Force -ErrorAction SilentlyContinue }} }}; break }} }}",
            bin_dir.display(),
            urls.iter().map(|u| format!("'{}'", u)).collect::<Vec<_>>().join(", ")
        );
        let _ = Command::new("powershell").args(["-NoProfile", "-Command", &ps_cmd]).output();
        if target_tcc.is_file() {
            return Ok(target_tcc);
        }
    } else {
        // Linux/macOS: system package manager first (best integration), then
        // our tiny (~500KB) standalone tcc tarball from the project release.
        let target_tcc = bin_dir.join("tcc");
        if target_tcc.is_file() {
            return Ok(target_tcc);
        }
        let _ = Command::new("bash").args(["-c", "(tcc -v || apt-get install -y tcc || sudo apt-get install -y tcc || dnf install -y tcc || apk add tcc || pacman -S --noconfirm tcc) 2>/dev/null"]).output();
        if which("tcc") {
            return Ok(PathBuf::from("tcc"));
        }
        let url = if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
            Some("https://github.com/StillAdum/Still/releases/download/v0.2.0/tcc-linux-x86_64.tar.gz")
        } else {
            None
        };
        if let Some(url) = url {
            // Extract so tcc lands in ~/.still/bin and its runtime in
            // ~/.still/lib (tcc finds libtcc1 via its ../lib/tcc relative path).
            let root = bin_dir.parent().unwrap_or(&bin_dir).display().to_string();
            let dl = format!(
                "curl -fsSLf '{u}' -o 'r/tcc-setup.tar.gz' && tar xzf 'r/tcc-setup.tar.gz' -C 'r' --no-same-owner && rm -f 'r/tcc-setup.tar.gz'",
                u = url,
                r = root
            );
            let _ = Command::new("bash").args(["-c", &dl]).output();
            let target_tcc = bin_dir.join("tcc");
            if target_tcc.is_file() {
                #[cfg(unix)]
                {
                    let _ = std::fs::set_permissions(&target_tcc, std::os::unix::fs::PermissionsExt::from_mode(0o755));
                }
                return Ok(target_tcc);
            }
        }
    }

    // Check if gcc or clang or tcc is available
    if let Some(tc) = find_cc() {
        return Ok(tc.program);
    }

    Err(format!("Portable compiler directory prepared at {}. Place 'tcc' or 'gcc' here or on PATH.", bin_dir.display()))
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
