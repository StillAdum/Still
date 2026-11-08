# Still v0.2.0 — Windows setup

**Still** is a compiled language that reads like Python and runs like C.
`fib(35)` matches `gcc -O2` C. Indentation syntax, type inference, refcounted
strings/lists/dicts (no GC pauses), value structs, `@soa` struct-of-arrays for
game hot loops, `extern "C"` escape hatch.

## Install on Windows (2 minutes)

### Step 1 — get the zip

Download **`still-v0.2.0-windows-x64.zip** (attached below) and extract it.

### Step 2 — install the compiler

Open the extracted folder in PowerShell and run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

This copies `still.exe` to `%LOCALAPPDATA%\Programs\Still` and puts it on your
PATH. No admin rights needed. Open a **new** terminal and check:

```powershell
still version
still run examples\fizzbuzz.strc
```

### Step 3 — a C compiler (one-time)

Still compiles through C, so it needs one of:

- **Visual Studio Build Tools 2022** (recommended, free):
  <https://visualstudio.microsoft.com/visual-cpp-build-tools/> → run installer →
  check **"Desktop development with C++"** → Install. Still finds it
  automatically — no developer prompt needed.
- or **WinLibs GCC** (<https://winlibs.com/>) with its `bin` folder on PATH.

### Step 4 — VS Code extension (optional)

```powershell
code --install-extension still-lang-0.2.0.vsix
```

Gives you highlighting, autocomplete, live error checking, **Ctrl+Alt+R** to
run, and **F5** to debug with breakpoints.

## What's in the zip

| file | what it is |
| --- | --- |
| `still.exe` | the compiler (prebuilt, no Rust needed) |
| `install.ps1` / `uninstall.ps1` | PATH installer / remover |
| `SETUP-WINDOWS.md` | this guide, extended (troubleshooting, command reference) |
| `examples\` | fizzbuzz, fib, mandelbrot, particles, and more — the language tour |

## Try it

```
name = "world"
print("hello, {name}!")

func fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

print("fib(25) = {fib(25)}")

parts = {"cpu": 4, "gpu": 1}
print(keys(parts))
```

```powershell
still run hi.strc
still build hi.strc -o hi.exe --release
```

## Troubleshooting

- **`no C compiler found`** → install VS Build Tools with the C++ workload
  (step 3), or run from the "x64 Native Tools Command Prompt for VS 2022".
- **`still` not recognized** → open a new terminal after installing.
- **SmartScreen blocked the download** → right-click the zip → Properties →
  Unblock → extract.

Full details: `SETUP-WINDOWS.md` inside the zip.

## Source

The compiler is written in Rust and emits C11. Build from source with
`cargo build --release`. CI runs the full conformance suite on both Linux
(gcc) and Windows (MSVC) for this tag.
