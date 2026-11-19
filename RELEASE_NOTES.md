# Still v0.6.3 — net header hardening

`still_net_header(id, name)` no longer reads the whole raw response-header
block into a wide buffer and scans it. The backend now queries only what it
needs per header (currently Content-Type, via WINHTTP_QUERY_CONTENT_TYPE)
and synthesizes the stored line. This removes the flaky heap read that could
crash a successful large response on Windows CI runners (a 200 with a big
body occasionally crashed where a 403 never did — the raw-headers buffer was
the difference between the paths). Requests and bodies are unchanged.

# Still v0.6.2 — two refcount/resolution fixes found by a real app

Wingram (a desktop client written entirely in Still) exercised paths the
conformance suite had not, and turned up two emitter bugs plus one checker
gap. All three are fixed here; the full conformance suite and every example
stay green, and Wingram's headless selftest passes on the fixed compiler.

- **Refcount underflow on element stores through struct fields.**
  `obj.items[k] = v` materialized the container field into a temporary and
  released it at statement end — without ever retaining it. Every such store
  decremented the container's refcount, so a dict/list held in a struct field
  was freed mid-program once enough stores ran (values came back as garbage
  keys, then crashes). `materialize_place_read` now retains the borrowed read
  so the statement-end release balances.
- **Latent double-free on boxed returns** (the adapt_ret hand-off released
  the consumed temp without unregistering it; the scope-end temp sweep
  released it again). This shipped in v0.6.0's window and is included here in
  the release notes for completeness — v0.6.0 and v0.6.1 already carry it.
- **Checker: path-form `use` now resolves to the file stem.**
  `use "dir/file.strc"` loads and registers under the stem `file`, but the
  checker still looked the module up by the raw target string, so any
  multi-directory project failed to typecheck with "unknown module". The
  checker now resolves path-form targets to the stem (matching the loader),
  with the same behavior for aliases and named imports.

# Still v0.6.1 — filesystem builtins

Two small universal additions on top of v0.6.0, driven by real applications
(Wingram's mod loader needs both):

- **`read_dir(path) -> [str]`** — lists names in a directory (skips `.` and
  `..`). Returns an empty list when the folder cannot be opened, so a missing
  folder reads as "nothing installed". Windows uses FindFirstFileA; POSIX
  uses opendir/readdir.
- **`file_exists(path) -> bool`** — an existence probe that does not panic on
  missing files (GetFileAttributesA / stat).

# Still v0.6.0 — universal platform libraries (net / gui / audio / media / json)

**Still** is a compiled language that reads like Python and runs like C.
v0.6.0 turns four typechecked-only stdlib stubs into real, working libraries
and adds a pure-Still JSON module — all universal, none app-specific. The
backends are Windows-first (per Addendum A) and every function degrades
gracefully on other platforms (`available()` returns false), so the same
program typechecks and runs everywhere.

## What's new in v0.6.0

- **`net` — real HTTP/HTTPS client.** `net.get(url)` / `net.post(url, body)`
  / `net.request(method, url, headers, body, timeout_ms)` run through WinHTTP
  with TLS, timeouts, redirect following and automatic gzip/deflate
  decompression. Headers are one `"Name: value"` per line. Responses come
  back as a `Response` value (`ok`, `status`, `body`, `error`,
  `content_type`). Also: `url_encode`, `url_decode`, `parse_qs`. Zero new
  link-time dependencies — winhttp.dll is resolved at run time (the same
  dlopen pattern the `py` bridge uses).
- **`gui` — real windowing + immediate-mode rendering.** `gui.open_window`
  opens a Win32 window (GDI double-buffered backbuffer). Frames are
  `begin_frame` -> draw -> `end_frame`. Primitives: `fill_rect`,
  `fill_round`, `fill_circle`, `stroke_circle`, `text`, `text_bold`,
  `text_width`, `draw_image` (with the media module), `clip_push` /
  `clip_pop` for scroll regions. Input: mouse position/click, wheel,
  per-key `key` / `key_pressed` (VK_* constants defined), `text_input` for
  typing. DefWindowProc handles the rest; no message-thread juggling.
- **`audio` — WAV playback + melody synthesis.** `audio.play_file(path)`
  plays WAV files; `audio.play_melody("440:500,0:100,660:500")` synthesizes
  a 16-bit 22050 Hz WAV in memory (per-note 8 ms fades, rests with `0:`) and
  plays it via winmm. Playback is async and single-channel;
  `audio.stop()` silences.
- **`media` — image loading via GDI+.** `media.load(path)` opens
  JPEG/PNG/GIF/BMP/ICO/TIFF and returns an opaque handle; `media.width`,
  `media.height`, `media.free`; render with `gui.draw_image` (high-quality
  bicubic). gdiplus.dll is resolved at run time.
- **`json` — new module, pure Still.** Strict RFC 8259 parser producing
  `any` values (dicts/lists/str/int/float/bool/none) with byte-offset error
  reporting, full `\uXXXX` escape support including surrogate pairs,
  `stringify` / `stringify_pretty`, and panic-free accessors (`get_str`,
  `get_int`, `get_float`, `get_bool`, `get_list`, `get_dict`, `get_at`)
  built on the panic-free `key in dict` membership test.
- **New builtin: `dict_key_at(v, i)`** — the i-th key of a string-keyed dict
  held in an `any`, pairing with `len(v)` to iterate `any`-dict keys without
  panicking. Backed by `still_dict_key_at` in the C runtime.
- **Memory-safety fix in the emitter.** `adapt_ret`'s boxed-return hand-off
  (returning a dict/list/str temp as `any`) released the temp but never
  unregistered it, so the scope-end temp sweep released it a second time — a
  latent double-free on every such return path. The temp is now unregistered
  at the hand-off. Found by the new JSON round-trip tests; conformance stays
  green.
- **Examples:** `json_tour` (runs everywhere), `net_demo`, `audio_demo`
  (graceful skip without the backend), `gui_demo`, `media_demo` (guarded by
  `STILL_RUN_GUI_DEMO=1` so CI stays headless-safe).

## Platform notes (honest scope)

- Windows gets real backends (WinHTTP/GDI/winmm/GDI+), matching the
  Windows-x64-first addendum. On Linux/macOS the modules typecheck and run;
  `available()` returns false and calls return neutral errors instead of
  crashing — a POSIX/OpenSSL backend is the natural follow-up.
- `gui` is a backend, not a widget toolkit: layout/widgets are the
  application's job (see the gui_demo for the immediate-mode pattern).
- Audio playback is single-channel (a new `play_*` stops the previous one).

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
