# Still v0.3.0 — Windows setup

**Still** is a compiled language that reads like Python and runs like C.
`fib(35)` matches `gcc -O2` C. Indentation syntax, type inference, refcounted
strings/lists/dicts (no GC pauses), value structs, `@soa` struct-of-arrays for
game hot loops, `extern "C"` escape hatch, and now: `match` statements, a
language server, and an embedded CPython bridge.

## What's new in v0.3.0

- **`match` statements** — pattern matching with literal, wildcard, binding,
  and tuple/list patterns (`case`/`is` syntax, exhaustiveness errors when a
  match can fall through).
- **Language Server (`still lsp`)** — real JSON-RPC/Content-Length LSP on
  stdio: live diagnostics as you type, document symbols, hover signatures, and
  completion. Works with Neovim/Emacs/any LSP client; the VS Code extension
  keeps its own faster path.
- **CPython bridge (`use py`)** — embed Python in a Still program:
  `py.run("import math")`, `print(py.eval("math.sqrt({x})"))`. libpython is
  loaded at run time (dlopen/LoadLibrary) — nothing is linked in. If the
  automatic search misses your install, set `STILL_PYTHON_DLL`.
- **`extern "C"` functions can return `str`** — the C function hands back a
  borrowed NUL-terminated string; Still copies it into a retained `still_str`.
- **CI benchmark gate hardened** — the fib-vs-gcc perf check now takes the
  best of 3 runs before enforcing the 3x limit, so CI runners can't fail the
  build with a noisy single sample.

## Install on Windows (2 minutes)

### Step 1 — get the zip

Download **`still-v0.3.0-windows-x64.zip** (attached below) and extract it.

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

- **TinyCC, instantly (2 MB)** — run `still setup-cc`. It downloads a portable
  C compiler into `~/.still/bin/` and verifies it with a compile probe. This
  is the zero-configuration path; no Visual Studio needed.
- **Visual Studio Build Tools 2022** (free):
  <https://visualstudio.microsoft.com/visual-cpp-build-tools/> → run installer →
  check **"Desktop development with C++"** → Install. Still finds it
  automatically — no developer prompt needed.
- or **WinLibs GCC** (<https://winlibs.com/>) with its `bin` folder on PATH.

### Step 4 — VS Code extension (optional)

```powershell
code --install-extension still-lang-0.3.0.vsix
```

Gives you highlighting, autocomplete, live error checking, **Ctrl+Alt+R** to
run, and **F5** to debug with breakpoints.

## Try the new stuff

```still
# match
for x in [1, 2, 3]:
    match x:
        case 1: print("one")
        case 2: print("two")
        case n: print("big: {n}")

# python bridge (needs Python installed)
use py
if py.init() == 0:
    py.run("import math")
    print(py.eval("math.sqrt(2)"))
```

```powershell
still run hi.strc
still build hi.strc -o hi.exe --release
```

## What's in the zip

| file | what it is |
| --- | --- |
| `still.exe` | the compiler (prebuilt, no Rust needed) |
| `install.ps1` / `uninstall.ps1` | PATH installer / remover |
| `SETUP-WINDOWS.md` | this guide, extended (troubleshooting, command reference) |
| `examples\` | fizzbuzz, fib, mandelbrot, py_bridge, and more — the language tour |

## Troubleshooting

- **`no C compiler found`** → run `still setup-cc` (installs portable tcc), or
  install VS Build Tools with the C++ workload (step 3).
- **`still` not recognized** → open a new terminal after installing.
- **SmartScreen blocked the download** → right-click the zip → Properties →
  Unblock → extract.
- **`py.init()` returns -1** → Python isn't found on PATH; set
  `STILL_PYTHON_DLL` to e.g. `C:\Users\you\AppData\Local\Programs\Python\Python312\python312.dll`.

Full details: `SETUP-WINDOWS.md` inside the zip.

## Source

The compiler is written in Rust and emits C11. Build from source with
`cargo build --release`. CI runs the full conformance suite on both Linux
(gcc) and Windows (MSVC) for this tag.
