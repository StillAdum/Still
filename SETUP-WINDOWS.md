# Still on Windows — Setup

Still is a compiled language that reads like Python and runs like C. This guide
gets it running on Windows 10/11 (64-bit) in seconds — with NO heavy C++ development kits required!

## 1. Requirements

- Windows 10 or 11, 64-bit
- The Still compiler: `still.exe`
- A C compiler — Still generates clean, standard C11:
  - **No Visual Studio / C++ Build Tools required!** Still does not compile C++; it compiles pure C.
  - **Option A (Instant, 2MB): TinyCC (TCC)**
    Run `still setup-cc` to automatically download a 2MB standalone C compiler directly into `~/.still/bin/`.
  - **Option B (Recommended for Production): WinLibs GCC or Clang**
    Download the portable GCC zip from <https://winlibs.com/>, extract it, and put its `bin` folder on your PATH.
  - **Option C: Visual Studio Build Tools (Optional)**
    If you already have MSVC installed, Still detects `cl.exe` automatically.
- Total footprint is well under 100MB (far below the 2GB budget!). Rust is **not** required.

Still finds your C compiler automatically: checks `~/.still/bin/` (TCC/GCC), system PATH (tcc, clang, gcc, cc, zig), or MSVC if present.

## 2. Install the compiler

1. Download and extract `still-v0.4.0-windows-x64.zip`
2. Open the extracted folder in PowerShell and run:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\install.ps1
   ```

   This copies `still.exe` to `%LOCALAPPDATA%\Programs\Still` and adds that
   folder to your user PATH. No admin rights required. To remove it later, run
   `uninstall.ps1` the same way.

3. Open a **new** PowerShell or CMD window and check:

   ```powershell
   still version
   ```

4. Compile and run something:

   ```powershell
   still run examples\fizzbuzz.strc
   still build examples\fib.strc -o fib.exe --release
   .\fib.exe
   ```

## 3. The language in 30 seconds

Save this as `hi.strc`:

```
name = "world"
print("hello, {name}!")

func fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

print("fib(25) = {fib(25)}")

nums = [5, 3, 8, 1]
nums.push(2)
for i, n in enumerate(nums):
    print("{i}: {n}")

parts = {"cpu": 4, "gpu": 1}
print(keys(parts))

type Player:
    name: str
    hp: int = 100

    func hurt(self, dmg: int):
        return Player(self.name, self.hp - dmg)

p = Player("ana")
p = p.hurt(30)
print("hp left: {p.hp}")
```

Run it:

```powershell
still run hi.strc
```

Build an optimized executable:

```powershell
still build hi.strc -o hi.exe --release
```

More examples ship in the `examples` folder of the zip (fizzbuzz, mandelbrot,
particles, string/data/module tours). The `still new myproj` command scaffolds
a project.

## 4. VS Code extension

The release also has `still-lang-0.4.0.vsix`.

1. Install the compiler first (step 2), so `still` is on PATH
2. Install the extension:

   ```powershell
   code --install-extension still-lang-0.4.0.vsix
   ```

   (or VS Code → Extensions view → `···` → **Install from VSIX...**)

3. Open any `.strc` file. You get:
   - syntax highlighting
   - autocomplete (symbols from your file)
   - error checking while you type
   - **Ctrl+Alt+R** runs the current file
   - **F5** builds with debug info and runs it under the debugger — set
     breakpoints in the editor, execution stops and reports the Still
     file:line

If VS Code cannot find the compiler, set
**Settings → Extensions → Still → Compiler Path** to the full path of
`still.exe` (default install: `%LOCALAPPDATA%\Programs\Still\still.exe`).

## 5. Troubleshooting

**`no C compiler found`**
Install Visual Studio Build Tools with the *Desktop development with C++*
workload (link in step 1), or put WinLibs gcc on PATH, then retry. If it is
installed and still not found, run from the
"x64 Native Tools Command Prompt for VS 2022" (Start menu).

**`still` is not recognized**
Open a new terminal after installing (PATH changes only apply to new windows),
or sign out and back in.

**Windows blocked the zip / SmartScreen**
Right-click the zip → Properties → check **Unblock** → OK, then extract.

**Antivirus flags the compiled exe**
Your program is a brand-new unsigned executable in `%TEMP%`; this is normal for
C-compiled output. Build with `still build ... -o myprog.exe` and run it from
its own folder.

**Where are the temporary build files?**
`%TEMP%\still_build_*` — generated C, runtime and objects live there. Delete
them freely.

## 6. Command reference

```
still run FILE.strc [--release] [-- args...]
still build FILE.strc -o OUT.exe [--release] [--keep-c]
still check FILE.strc
still emit-c FILE.strc [-o OUT.c] [--release]
still test DIR
still new NAME
still version
```
