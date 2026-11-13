# Still v0.4.0 — Windows setup

**Still** is a compiled language that reads like Python and runs like C.
`fib(35)` matches `gcc -O2` C. Indentation syntax, type inference, refcounted
strings/lists/dicts (no GC pauses), value structs, `@soa` struct-of-arrays for
game hot loops, `extern "C"` escape hatch, and now: `any` values, real
tuple/list `match` patterns, verified debug line info with breakpoints, and
LSP find-references/rename.

## What's new in v0.4.0

- **`any` end-to-end** — one type for heterogeneous data: `mixed = [1, "two",
  3.5, true]` typechecks, `type_of` reports the runtime type, and
  arithmetic, comparisons, casts, membership and truthiness all dispatch on
  it (`x is none` narrows too). Boxable scalars (int, float, bool, str) and
  containers (lists, dicts) carry a runtime tag. Covered by the conformance
  suite and the garbage-input fuzzer.
- **`match` tuple/list patterns (for real this time)** — destructuring case
  arms: `case (a, b):`, `case [x, y]:`, nested patterns like
  `case (a, (b, c)):`, wildcards mixed with literals (`case [_, 20, _]:`),
  and all-literal container patterns (`case (3, "hi"):`). List patterns check
  length at run time, so `case [a, b]:` on a 3-list simply falls through to
  the next arm. v0.3's notes promised these; v0.4 ships them, with conformance
  tests for nesting, fall-through, arity errors and subject-type errors.
- **Debug line info you can trust** — panics and breakpoints now report the
  real `.strc` line in every case we could construct: the generated C writes
  the `#line` mapping per statement instead of trusting a running counter,
  and `STILL_BREAKPOINTS` matching handles the compiler's build-dir-prefixed
  paths (full-path and basename forms, multiple entries, indented loop/if
  bodies). Verified end to end with `still run`; the VS Code DAP contract
  (basename entries + SIGTRAP stop) is unchanged.
- **String `+=` folding everywhere** — `s += t` and `s = s + t` on a `str`
  both fold to the in-place append, so string building in loops stays linear.
  Evidence: the emitted C contains two `still_str_append_inplace` sites and
  zero `still_str_cat` calls. (There is no string benchmark in the CI gate,
  so this ships with fold evidence rather than a bench number.)
- **LSP find-references and rename** — `still lsp` now answers
  `textDocument/references` (with `includeDeclaration`) and
  `textDocument/rename` (workspace edit over every identifier token of the
  name; invalid names report an error). Matches come from the real lexer, so
  string literals and comments never match.

## Install on Windows (2 minutes)

### Step 1 — get the zip

Download **`still-v0.4.0-windows-x64.zip`** (attached below) and extract it.

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
code --install-extension still-lang-0.4.0.vsix
```

Gives you highlighting, autocomplete, live error checking, **Ctrl+Alt+R** to
run, and **F5** to debug with breakpoints.

## Try the new stuff

```still
# any: one list, every shape
mixed = [1, "two", 3.5, true]
for x in mixed:
    print(x, type_of(x))

# match: destructuring, nesting, wildcards
p = (3, "hi")
match p:
    case (0, b):
        print("zero")
    case (3, greeting):
        print("hello", greeting)

xs = [10, 20, 30]
match xs:
    case [a, b]:
        print("two")
    case [a, b, c]:
        print("three", a, b, c)

# string building stays linear
s = ""
for w in ["ab", "cd", "ef"]:
    s += w
print(s)
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
