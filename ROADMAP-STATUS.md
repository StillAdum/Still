# ROADMAP-STATUS — v0.3.0

Status of Still against `roadmap.md` ("Custom Programming Language: Implementation Roadmap", 13 phases + 2 addenda), written per that document's own rule #6 ("report status against them honestly, including what was skipped or stubbed") and rule #10. Dates: v0.3.0, November 2026. Nothing here is aspirational — every ✅ has a test or a runnable demo behind it in the repo.

## Verdict in one paragraph

Still v0.3.0 is a working Phase-1–4 language with a real Phase-6 C-ABI hub, a real Phase-7 tooling slice (LSP + DAP + release automation), and a first Phase-6.5 Python bridge. It is **not** a Phase-9/10/11 language: there is no LLVM/Cranelift backend, no DWARF/PDB stepping, no threads, no `core`/`alloc` split, no kernel story, and several stdlib modules are typechecked-only stubs. Those gaps are listed phase by phase below instead of being papered over.

## Phase 1: Front End — ✅ mostly shipped (for the implemented language)

Shipped: hand-written lexer (indentation blocks, `{expr}` interpolation capture, spans), recursive-descent parser with error recovery, AST with spans, diagnostics with did-you-mean, name resolution + modules (`use`, aliases, embedded stdlib), type checker with full inference, implicit generics + monomorphization, `T?` nullable with flow-unwrap, `match` statements (literal/wildcard/binding patterns), conformance suite (50/50 green on gcc + tcc, plus MSVC in release CI), parser fuzzer test in cargo test.

Not shipped (roadmap says build, we say deferred): formal EBNF grammar document; error codes and lint levels; traits/interfaces; enums/ADTs with pattern matching beyond literal cases; **closures** (see Deferrals); const evaluation; a general attribute system (only `@soa` and `@export` exist); the 500-program conformance target (50 today); a 1-hour fuzzer run (cargo fuzz test is short).

## Phase 2: Middle End and Back End — ⚠️ C emitter only

Shipped: checked IR (Tx/X/CS) → C11 emitter with refcounted ownership emission; compiles through gcc/clang/MSVC/TinyCC; `-O2` release builds; debug mode carries file:line into panics and breakpoint traps (`STILL_BREAKPOINTS`, used by the VS Code debugger adapter).

Deferred (this is the phase's backbone): HIR/MIR/SSA + verifier; optimization passes (inlining/DCE/escape analysis — gcc does this on emitted C today); LLVM or Cranelift backend; bytecode VM; **DWARF/PDB debug info — real debugger stepping is explicitly deferred** (breakpoints exist, stepping does not); incremental/query-based compilation; linker integration and cross-compilation triples; low-level codegen controls (struct layout attrs, calling conventions, inline asm, naked functions).

## Phase 3: Runtime and Memory — ⚠️ partial

Shipped: refcounted runtime (strings/lists/dicts/SoA — Python sharing semantics, no GC pauses), panics with file:line, checked div/mod/index/overflow in debug, dynamic library loading at run time (the Python bridge's dlopen/LoadLibrary loader), `STILL_BREAKPOINTS` traps.

Deferred: allocator trait/global swap, arena/bump/pool allocators; the `core`/`alloc`/`std` layer split (kernel work depends on it — therefore Phase 9 stays closed); replaceable panic handler; stack traces; destructors/`defer`/weak refs; sanitizer hooks; leak detector.

## Phase 4: Core Standard Library — ⚠️ honest split

Shipped and real: math, sort, random, time, io, os, vec, pathfinding, units, fixed, text + the game-oriented modules that ship real implementations; 46 modules typecheck clean; strings with interpolation/slicing/methods; lists/dicts/tuples; `@soa` SoA collections.

Typechecked-only stubs (compile today, open real handles/resources later — this is worklog item 10): gui, audio, net, web, ipc, media, directx and friends. Not shipped: big integers, f16/f128, regex engine, Unicode normalization/segmentation, iterators with adapters, SIMD. The benchmark suite tracking stdlib regressions is the CI fib-vs-C gate (best of 3, ≤3x), not a full perf suite.

## Phase 5: OS, IO, Time, Concurrency — ❌ mostly deferred

Shipped: file read/write, env vars, argv, clock, sleep, `run_cmd`. Everything else — processes/pipes/signals, compression/archives, **threads, async, atomics, channels** — deferred. Concurrency absence is the single biggest runtime gap; it is also why the Python bridge is documented main-thread-only.

## Phase 6: Interoperability — ✅ the hub ships; one bridge ships

- **6.1 C ABI hub: shipped.** `extern "C"` declarations (int/float/bool/str params incl. variadics; int/float/bool/**str** returns), `@export` for exposing Still functions as plain C symbols, C prototypes for Still functions in emitted headers of exported code, runtime convention documented (str args borrow `->bytes`, str returns copy into a retained `still_str`).
- **6.5 Python: shipped (forward, embedded).** `use py` — CPython embedded at run time via dlopen/LoadLibrary of libpython (zero link deps, `STILL_PYTHON_DLL` override), `py.init()/py.run()/py.eval()` over the C ABI hub, shared `__main__` state, exceptions surfaced as Still panics. Deferred: NumPy zero-copy buffer views, building Still code as Python extension modules (reverse direction), pip/PyPI resolver integration, type-stub generation.
- **6.2 C++ / 6.3 Rust / 6.4 C#: not started.** The roadmap's own rule #8 says say so plainly: no libclang binding generator, no hostfxr/.NET hosting. The C ABI hub is the workaround path (any of those ecosystems reachable through flat `extern "C"` shims authored by hand).
- **6.6 Cross-cutting:** dlopen dynamic loading shipped (the py loader); FFI type-mapping table documented in worklog; ownership annotations are conventions, not checker-enforced.

## Phase 7: Tooling and Ecosystem — ✅ the shipped slice

- **LSP: shipped** (`still lsp`) — JSON-RPC 2.0 with Content-Length framing on stdio, full text sync, publishDiagnostics on open/change/close, documentSymbol, hover (real signatures), completion (keywords + symbols), graceful shutdown, survives garbage input. Integration-tested against the real binary. No rename/format/find-references yet.
- **DAP: shipped for VS Code** (breakpoint traps + dap.js; stepping blocked on DWARF, see Phase 2).
- Shipped: TextMate grammar + VS Code extension, `still test` runner, CI (conformance + examples + panic-path check + benchmark gate + MSVC job), release automation (tag → zip + vsix built on windows-latest, portable tcc attached).
- Deferred: package manager/registry/lockfiles, formatter, linter, REPL, doc generator/docs site, profilers, tree-sitter grammar, JetBrains/Neovim plugins (Neovim works today via the LSP binary), self-hosting.

## Phase 8: Networking, Data and Security Libraries — ❌ deferred

Typechecked stubs exist (net/web/server/serialize); none of it is claimed as working. No TLS, no SQLite, no crypto beyond hashing helpers.

## Phase 9: Systems and Kernel-Level Support — ❌ not started

Per the roadmap's own dependency chain this needs the Phase-3 `core` split and Phase-2 codegen controls first. Honest status: closed until those exist.

## Phase 10: Graphics, Audio, Input, Windowing — ⚠️ Windows-first extern modules

win32/directx/media/gui/audio modules bind real Win32/DirectX C symbols (authored and committed; exercisable on Windows boxes), examples render via them. No cross-API RHI, no shader cross-compiler, no Metal/Vulkan/WebGPU. Per Addendum A's "Windows x64 first" this skew is intentional.

## Phase 11: Game Engine Foundations — ⚠️ patterns, not an engine

physics/pathfinding/procgen/particles/spatial/arena/game_systems modules + example programs (voxel_sandbox, platformer_2d, bullet_hell, physics_sandbox) demonstrate chunked worlds, spatial grids, particle systems. No ECS, no job system, no netcode lib (net_rollback module is typecheck-level).

## Phase 12: High-Level Application Support — ❌ not started

## Phase 13: Hardening and Launch — ⚠️ the CI part

Shipped: fuzz test (garbage input never panics/hangs), conformance gate on Linux gcc + tcc and Windows MSVC, benchmark regression gate, panic-path CI check. Deferred: sanitizer CI, security review, reproducible builds, signing/SBOM, LTS policy, complete docs/book.

## Addendum A (Windows-only scope) — ⚠️ honored in spirit

Windows is first-class: MSVC release CI, zip installer with PATH setup, portable tcc so end users need no VS Build Tools, Win32/DirectX bindings. Deviations: Still emits C11 and compiles with an available C compiler instead of shipping its own LLVM/lld toolchain and generated import libraries; PDB emission deferred with Phase 2.

## Addendum B (foreign modules, native-style) — ✅ shape proven with py

`use py` follows the three-layer design: raw layer hidden in `stdlib/py.strc` (`still_py_*` externs), bridge layer in the C runtime (loader, GIL/error handling), native-style layer as `py.init/run/eval` with doc comments. The resolver/lockfile/PyPI automation is deferred.

## The two headline deferrals (why)

1. **`any`/dynamic type** — heterogeneous containers are the #1 Python-compat gap, but bolting a dynamic value onto a refcounting emitter that specializes every operation per static type would wreck the emitter. Needs a tagged `still_value` runtime design done properly.
2. **Closures** — capture semantics interact with the ownership/refcount emission (who owns the environment, when it is released). A half-implementation would leak or double-release. Needs the environment-as-value design first.

Both are worklog next-steps with ownership conventions already documented; neither is stubbed, and no release notes claim them.

## Phase-exit criteria scorecard (roadmap's own criteria)

| Roadmap exit criterion | Status |
| --- | --- |
| Phase 1: 500-program conformance, 1h fuzzer | ❌ 50 programs, short fuzzer |
| Phase 2: native exes on 3 OSes, debugger steps | ⚠️ exes on Windows/Linux via C toolchains; macOS untested; stepping deferred |
| Phase 3: freestanding program, custom entry/panic | ❌ |
| Phase 4: 90% coverage, bench suite, doc-tests | ❌ partial (bench gate only) |
| Phase 5: multithreaded tool on 3 OSes | ❌ no threads |
| Phase 6: call C/C++/Rust/.NET/Python in one program | ⚠️ C ✅ Python ✅; C++/Rust/.NET via manual C shims only |
| Phase 7: ecosystem | ⚠️ LSP/DAP/CI/release ✅; package manager/REPL/docs ❌ |
| Phase 9: signed driver in a VM | ❌ |
| Phase 11: voxel engine demo | ⚠️ examples demonstrate patterns, not an engine |
