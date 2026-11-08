# Still for Visual Studio Code

Syntax highlighting, autocomplete, error checking, running and debugging for the
[Still programming language](https://github.com/) (`*.strc` files).

## 1. Install the Still compiler

The extension drives the `still` compiler, so install it first.

### From source (the usual way)

You need [Rust](https://rustup.rs) (1.75 or newer) and a C compiler (gcc or clang).

```bash
git clone <this repository>
cd Still
cargo build --release
```

That produces `target/release/still`. Put it on your PATH:

- **Linux/macOS**: `sudo cp target/release/still /usr/local/bin/`
  (or add `export PATH="$PATH:/path/to/Still/target/release"` to your shell profile)
- **Windows**: copy `still.exe` somewhere on PATH, or add its folder to PATH.

Check that it works:

```bash
still version
echo 'print("hi from still")' > hi.strc
still run hi.strc
```

### Verify inside VS Code

Open the command palette (`Ctrl+Shift+P`) and run **Still: Check File** on any
`.strc` file. If you see `still: cannot start the still compiler`, the compiler
is not on PATH — set **Settings > Extensions > Still > Compiler Path** to the
full path of the binary.

## 2. Install this extension

- **From source (no tools needed)**: copy this `vscode` folder anywhere, then in
  VS Code run **Extensions: Install from VSIX**... or simply:
  `code --extensionDevelopmentPath=/path/to/Still/vscode`
  (or open this folder and press F5 to launch an Extension Development Host).
- **Packing a .vsix** (requires Node.js):
  ```bash
  npm install -g @vscode/vsce
  cd vscode
  vsce package     # creates still-lang-0.2.0.vsix
  code --install-extension still-lang-0.2.0.vsix
  ```

## 3. Features

| Feature | How to use |
| --- | --- |
| Syntax highlighting | automatic for `.strc` files |
| Errors as you type | automatic; also **Still: Check File** (Problems panel) |
| Autocomplete | keywords, builtins, and every function/type/variable in your file |
| Hover docs | hover any builtin or symbol |
| Go to definition | F12 on a function/type/variable in the same file |
| **Run** | **Still: Run File**, `Ctrl+Alt+R`, or the ▶ button (top right) |
| **Debug** | **Still: Debug File**, F5, or the bug button |

## 4. How debugging works

Pressing F5 builds your file in **debug mode** (runtime checks on, `-g`, panic
messages point at `.strc` lines) and runs it in the Debug Console:

- program output and panics stream into the Debug Console
- red breakpoints are honored: the program halts at the line and reports it
  (the Still debug runtime raises a trap at the breakpoint line)
- the whole program runs under the checked runtime, so out-of-range indexes,
  integer overflows, missing dict keys and failed `check()` calls all stop with
  a precise `file:line` message — this is the fastest way to find bugs

## 5. A quick tour of the language

```python
# still is indentation-based, types are inferred
name = "world"
print("hello, {name}!")

func fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

for i in range(10):
    print(fib(i))

nums = [3, 1, 2]
nums.push(4)
ages = {"ana": 31}
for k, v in pairs(ages):
    print(k, v)

type Point:
    x: float = 0.0
    y: float = 0.0
    func mag(self) -> float:
        return (self.x ** 2 + self.y ** 2) ** 0.5

use math
print(math.sqrt(2.0))
```

## 6. Settings

| Setting | Default | Meaning |
| --- | --- | --- |
| `still.compilerPath` | `still` | path to the compiler binary |
| `still.diagnosticsOnSave` | `true` | check on save/open |
| `still.diagnosticsOnType` | `true` | check while typing (350 ms debounce) |

## 7. Troubleshooting

- **"cannot start the still compiler"** — the compiler is not on PATH; set
  `still.compilerPath` to the full binary path.
- **No errors shown while typing** — the compiler must be on PATH for the
  checker; run **Still: Check File** to see the reason.
- **F5 does nothing** — make sure the active editor is a `.strc` file and no
  other debug session is running.
