# Still — Worklog

State: **v0.2.0 — new Python/Lua-style syntax, full rewrite, all tests green; released as v0.2.0 with Windows installer + VSIX (GitHub Actions builds it on windows-latest).** Read this before touching anything. Token-dense on purpose.

## What Still is (v0.2)

Easy, Python/Lua-flavored compiled language. `*.strc` files. Compiler in Rust (zero deps), emits C11, compiles with gcc/clang. Type-inferred with implicit generics. Refcounted strings/lists/dicts (Python sharing semantics, no GC pauses). Structs are values; `@soa` makes struct-of-arrays collections for game hot loops. Reads like Python, runs like C: `fib(35)` matches `gcc -O2` C exactly (both ~14 ms on the dev box; CI enforces <3x).

v0.1 was Rust-like (`fn`/`let`/`match`/`Option[T]`/`defer`/pointers). The user ordered a **complete syntax redesign** ("like python and luau/lua KINDA"), so v0.2 is a front-to-back rewrite: new lexer (indentation), new parser, new checker, new emitter, new runtime, new tests. Old syntax is gone; nothing of it remains except the C-toolchain driver idea and the general pipeline shape.

## Layout

```
src/
  lexer.rs     indentation tokens, {{}} escapes, {expr} interpolation capture, ';' = newline
  parser.rs    recursive descent, Pratt expressions, error recovery w/ progress guarantee
  ast.rs       AST + TypeAnnot + Ty (inference types w/ Var cells) + Tx/X/CS (checked IR) + Program
  span.rs      spans, Diag, did-you-mean (levenshtein)
  check.rs     names, inference, monomorphization, flow-none, builtins, modules, externs, diagnostics
  emit_c.rs    checked IR -> C11; refcount emission; type registry (list/dict/struct/tuple/opt/soa/eq/print/tostr helpers)
  cc.rs        find cc/gcc/clang then cl; Windows: vswhere -> vcvars64.bat wrap via generated still_build.bat; cl runs with current_dir=tmp (objs land there), /Fe absolute
  lib.rs       driver: module loading ('use'), stdlib embedding, build/run/test/new, ide commands
  bin/still.rs CLI
  rt/          embedded C runtime (header + impl), compiled alongside generated code
  stdlib/      embedded Still modules: math, sort, random, time, io, os
conformance/   valid/ + invalid/ (.strc + .expected; tests are the syntax spec)
SETUP-WINDOWS.md + install.ps1/uninstall.ps1   Windows end-user install (zip ships these)
RELEASE_NOTES.md  becomes the GitHub release body (tag push)
examples/      showcase programs, double as docs
tests/         cargo tests: conformance runner, examples compile, fuzz (garbage in -> no panic/hang)
vscode/        extension: grammar, completion, hover, diagnostics, run/debug, DAP; README = setup instructions
.github/       ci.yml: build, cargo test, conformance, examples, panic check, bench vs C, extension check, windows job (MSVC, real gate, no continue-on-error)
               release.yml: tag v* -> windows-latest build -> conformance+examples on MSVC -> zip (still.exe+install.ps1+SETUP-WINDOWS.md+examples) + vsce vsix -> gh release create/upload
```

## Commands

```
cargo build --release
target/release/still run file.strc [--release] [-- args]
target/release/still build file.strc -o out [--release] [--keep-c]
target/release/still check file.strc
target/release/still emit-c file.strc -o out.c [--release]
target/release/still test conformance
target/release/still new myproj
target/release/still ide symbols file.strc   (JSON symbols for editors)
target/release/still ide check file.strc     (JSON diagnostics)
cargo test --release
```

## Language v0.2 (conformance/ + examples/ are the spec)

- Indentation blocks with `:`; one-line bodies (`if x: return 1`); `pass`; `;` separates statements.
- No declarations: `x = 5` infers. Annotation optional: `x: int = 5`, `nums: [int] = []`. Reassignment type-checked (`x = none` later widened via the `T?` placeholder rule).
- Types: `int`(i64) `float`(f64) `bool` `str` `[T]` `{K: V}` (int/str keys only) `(A, B)` tuples, `T?` nullable, named structs. Never mixed lists (`[1, "a"]` is an error; `[1, 2.5]` widens to float).
- `/` always float; `//` floor div (Python sign rules); `%` Python sign; `**` pow (int**nonneg-literal stays int); int/float mix widens; `and or not in is`; chained comparisons rejected with a hint; `a if c else b` ternary.
- Truthiness Python-style: none/false/0/0.0/""/[] falsy.
- `none` + `T?`: `x = none` makes a placeholder that widens to `T?` on first real value. Flow-unwrap: inside `if x:` / `if x is not none:` reads of `x` are unwrapped to `T` (checker rewrites reads to X::Unwrap).
- Functions: `func f(a, b):` — unannotated params = implicit generics, monomorphized per call-site types (instantiation cache keyed by resolved arg types). Defaults `b = "x"` must be const (type inferred from the default). `-> T` optional. Self-recursion with unknown return type works via the same-Var-cell→Int heuristic in arith; mutual recursion via compare/arith constraints. Uncalled generics are not checked (documented).
- Methods: `type P:` fields (with defaults) + `func m(self, ...)`. Call `p.m(...)`. Structs are VALUES (copy on assign; lists/dicts/strs/SoA are shared refs). Struct cycles through values rejected.
- `@soa type Part:` → `Part(cap)` refcounted parallel-arrays collection: `parts.push(x, y, ...)`, `parts.x[i]` read/write, `len(parts)`, `parts.clear()`, `for i in parts` (indexes). Fields must be int/float/bool/str.
- Builtins: print len push pop keys pairs str int float bool abs min max sum range enumerate reversed type_of input clock rand seed check fail exit ord chr read_file write_file get_env sleep_ms. Methods: list .push/.pop/.len/.clear; str .len/.upper/.lower/.trim/.contains/.starts/.ends/.split; dict .len/.clear; soa .push/.clear/.len.
- `for x in list|str|dict(keys)|range|soa`, `for i, x in enumerate(list)`, `for k, v in pairs(dict)` (also any list of 2-tuples). Loop var assignment is an error.
- Multi-assign/swap `a, b = b, a` (emits temp tuple unless all targets fresh), multi-return `return a, b`, tuple index `t.0`.
- `print` is Python-style: `print(nums)` → `[3, 1, 2]`, dicts/structs/tuples/nested all print; strings quote inside containers. `"{x}"` interpolation always on (`{{` escapes); containers interpolate via a string-builder family.
- Modules: `use math` (alias access `math.sqrt`), `use math: sqrt, pi` (imports names), `use "./path.strc"` or a local `mylib.strc`. Embedded stdlib: math (libm externs + gcd/lcm/clamp/lerp/sign), sort (generic quicksort + sorted), random (seeded, rand_range/shuffle/choice), time, io (read_lines/write_lines), os (env/run_cmd). Module top-level statements run once at start, in dependency order; `use` cycles are errors.
- extern "C": `func puts(s: str) -> int` incl. variadic `...` (str passes NUL-terminated bytes; variadic int passes int64 — use `%ld`). Only int/float/bool/str params; returns int/float/bool.
- `@export func name(...)` → non-static C symbol with the plain source name (non-generic only).
- Panics (always on): index bounds, missing dict key, div/mod by zero, pop empty. Debug-only: integer overflow checks (builtin `__builtin_*_overflow`), breakpoint traps (env `STILL_BREAKPOINTS="file:line,..."` → SIGTRAP at line, used by the VSCode debugger). Release: wrapping arithmetic, `-O2 -flto`.

## Memory model (do not change silently)

- str/list/dict/SoA are heap refcounted (`still_hdr { rc, destroy }`), shared by assignment — Python/Lua semantics. Structs/tuples are values; ref fields retained on copy (generated `_clone`/`_drop` per type), released at scope end/return/break paths.
- Emitter ownership discipline: every ref-typed expression result is an OWNED temp registered in `temps`; statement end releases temps; assignment moves the temp into the place (unregister); call args transfer to callee (params dropped at return); return materializes into a temp BEFORE local drops (use-after-free class of bug — see Gotchas); `And/Or/Cond` emit if/else with release of the untaken side; loop bodies drop iteration temps to a mark; loop vars own a retained element per iteration.
- Reference cycles leak (documented; no GC). Stress-verified: 1000 rounds x (200 strings + 200-list + 200-dict) flat ~1.9 MB RSS.

## Verified status (all green at commit time)

- 38/38 conformance (22 valid incl. soa/extern/modules/mandelbrot/pairs-loops/generics/nullable-flow, 16 invalid diagnostics).
- cargo test: conformance runner, all examples compile, release smoke, fuzz (3000 garbage parses no-panic, deep nesting, token bound).
- Bench: fib(35) release == gcc -O2 C (13.9 ms both on dev box; CI gate <3x).
- Memory stress flat; ASAN clean on the repro cases that found the bugs below.

## Bugs found and fixed during this rewrite (learn from these)

1. Return-after-free: `return retain(v1)` evaluated AFTER local drops → materialize return temps first, drop later (emit_c CS::Return).
2. Double-adapt in dict element assignment: value adapted twice (first moved the temp out of the registry, second saw a "borrow" and retained) → +1 leak per insert. Dict branch must reuse the outer `val`.
3. Top-level stray `Dedent` → parser infinite error loop (no progress). parse_program now force-skips Dedents and force-advances on no-progress iterations.
4. Parser stack overflow on deep nesting → expr depth guard 100 in parse_primary.
5. `x = {0}` is not C — aggregate zero needs `(T){0}` compound literal in assignments/returns (statics keep bare `{0}`).
6. Cross-TU arithmetic calls killed inlining (5x fib slowdown) → still_add/sub/mul/idiv/floordiv/imod are `static inline` in the header; `-flto` on the link.
7. Module `use` resolution ran per-module before all externs were registered (HashMap order) → two passes now (items first, then uses).

## Gotchas for the next AI

- The checker runs in passes: collect items (all modules) → collect uses → struct registry (+cycle check) → module inits (top-level code) → main body → eager-check non-generic fns → convert externs → finalize (unresolved-Var hunt with "annotate it" messages). Diags abort early between passes.
- Monomorphization: `instantiate()` creates the CFunc placeholder (ret cell shared with CFunc.ret) BEFORE checking the body — recursion resolves through `insts`. Methods use fname key `"Type.m"`; self type comes from the call's receiver; eager check passes `Ty::Named(qual)` for self.
- Type tags: `ty_tag` (ast.rs) is the single mangling source; ty_name is the user-facing name. List tags: elem tag only; tuple tag `T...`; dict `K_V`; nullable value types get `still_opt_TAG`; nullable refs are plain pointers (NULL = none).
- Emitter type registry: `need_list/dict/dict_keys_fn/tuple/opt/soa/struct/eqn` + `print_fn/repr_fn/tostr_fn` families register into `helpers` (dedup by key) and are PREPENDED to the output — register in dependency order (recursion handles it).
- Places: Place::Index on lists emits raw `->data[i]` writes via `place_deref_base`; dicts go through `still_dict_K_V_set` (takes ownership of key+value); SoaElem emits `->f_x[still_soa_index(i, len)]`. AugAssign reads the place then re-assigns (ref-elem aug on `a[i] += b` leaks one retain — known, rare, documented).
- `#line` remap drives panic/breakpoint messages; `last_line` bookkeeping in the emitter is approximate (off by a line or two inside multi-line statements).
- The runtime C (rt/) is embedded via include_str! and written next to generated code at build time; list_i/list_s are hand-instantiated in rt (non-static, shared); every other instantiation is a `STILL_DEFINE_LIST/DICT` macro in the generated file. Macro hooks: RETAIN/RELEASE/CLONE are `_v`-expressions; EQ/EQV are two-arg function refs.
- `still test` compares exact stdout for valid/, expected-substring for invalid/ (expected files are .trim()'d).
- VSCode extension is zero-dependency: `still ide check/symbols` JSON commands feed diagnostics/completion/hover; dap.js is a hand-rolled DAP server (launch → `still build` → spawn with STILL_BREAKPOINTS env; breakpoint hit = SIGTRAP, reported via output event; no continue-after-trap — honest limitation, documented).

## Known limitations (v0.2, honest list)

- No closures, nested functions, classes/inheritance, exceptions, match, step slices, chained comparisons, heterogeneous containers (need `any`), `global` keyword (assigning a global name inside a function shadows with a local — Python rule), argc/argv, threads, borrow checking. All are worklog-future items, not promises.
- Dict iteration order is hash order (deterministic per build, not insertion).
- `print`/`str()` of a `str?` prints the string or `none` (nested containers still quote strings).
- Aug-assign on a ref-typed list element (`xs[i] += s`) retains an extra ref (leak, not a crash).
- Windows: fully supported since v0.2.0 release — CI runs full conformance on MSVC; release zip built on windows-latest. Debug overflow checks use an _MSC_VER shim in still_rt.h (MSVC has no __builtin_*_overflow).
- CPython bridge from v0.1 was dropped in the redesign (the `extern "C"` hub is the path back; see roadmap Phase 6).

## Windows release notes (v0.2.0, how it ships)

- Repo: github.com/StillAdum/Still (delete+recreated fresh, single squashed commit, tag v0.2.0).
- Release = GitHub Release on tag: body from RELEASE_NOTES.md, assets still-v0.2.0-windows-x64.zip + still-lang-0.2.0.vsix, built by release.yml on windows-latest.
- zip layout: still.exe, install.ps1 (copies to %LOCALAPPDATA%\Programs\Still, user PATH), uninstall.ps1, SETUP-WINDOWS.md, examples/.
- End user needs VS Build Tools (C++ workload) or WinLibs gcc; still finds them itself (vswhere fallback when cl is not on PATH — works from any terminal, no dev prompt needed).
- MSVC gotchas fixed: __builtin overflow shims; cl objs go to tmp dir; /Fe forced absolute; vcvars wrapped in a generated .bat (quoting: never put trailing-backslash paths before a closing quote in bat/cmd lines — MSVCRT eats \" ).
- Doc gotcha learned: methods CANNOT mutate self (structs are values, self is a copy) — the idiom is `p = p.hurt(30)` returning a new struct; SETUP-WINDOWS.md sample is compiler-verified.
- Deleted v0.1 leftovers: conformance/py (old-syntax pyinterop, did not compile), examples_dbg (old Rust scratch).

## Next steps (priority order)

1. `any`/dynamic type for heterogeneous dicts (the #1 Python-compat gap).
2. String builder / `+=` folding for io.write_lines-style loops (quadratic today).
3. Borrow-free mutation story for structs in lists (`xs[i].x = v` works; `xs[i] = xs[i]` copies).
4. Closure capture-by-value; then stdlib sort/map/filter/each over closures.
5. LSP server binary (`still lsp` on JSON-RPC stdio) so Neovim/others get what VSCode has.
6. `global` keyword; argc/argv; step slices; match-on-int with exhaustiveness.
7. Bench suite in CI (SoA particle update vs C reference, fail >10% regression).
8. CPython bridge: `use py` with py.init/py.run/py.call over the C ABI hub.
9. Crash-only debugging → real stepping (needs DWARF work in the C backend).
