# Still v0.4.1 — Windows setup

**Still** is a compiled language that reads like Python and runs like C.
`fib(35)` matches `gcc -O2` C. Indentation syntax, type inference, refcounted
strings/lists/dicts (no GC pauses), value structs, `@soa` struct-of-arrays for
game hot loops, `extern "C"` escape hatch, `any` values, real tuple/list
`match` patterns, verified debug line info with breakpoints, and LSP
find-references/rename.

## What's new in v0.4.1

- **Real C prototypes for `extern "C"` declarations** — the generated C
  previously called externs with no declarations (implicit declarations).
  On 64-bit Windows that truncates integer arguments to 32 bits
  (`HWND_TOPMOST` = -1 arrived as 0xFFFFFFFF → "invalid window handle") and
  truncates pointer returns to `int` (`GetModuleHandleA` under gcc returned
  garbage, so `CreateWindowExA` failed outright). Behavior also differed
  randomly between tcc and gcc. The compiler now emits a prototype per
  extern — `int64_t` for ints/handles, `char*` for str params,
  `const char*` for str returns, `double` for floats — so Win32 GUI
  programs written in Still behave identically on every C compiler.
  Found the hard way: a whole raytraced voxel game rendered black because
  of this; after the fix its menu and gameplay blits verify pixel-perfect
  on screen under both tcc and gcc `-O3 -flto`.

## Install on Windows (2 minutes)

### Step 1 — get the zip

Download **`still-v0.4.1-windows-x64.zip`** (attached below) and extract it.

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
code --install-extension still-lang-0.4.1.vsix
```

Gives you highlighting, autocomplete, live error checking, **Ctrl+Alt+R** to
run, and **F5** to debug with breakpoints.

## Try the new stuff

```still
# extern "C" now emits real prototypes - Win32 works on any C compiler
extern "C":
    func MessageBoxA(hwnd: int, text: str, caption: str, flags: int) -> int

MessageBoxA(0, "Hello from Still", "v0.4.1", 0)
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
