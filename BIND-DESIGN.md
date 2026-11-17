# C-Header Binding Generator for Still

## Overview
Parses C header files and generates Still `extern "C"` declarations.
This is the gateway for C++, Rust, and C# libraries.

## Usage
```
still bind HEADER.h [-o output.strc] [--prefix PREFIX]
```

## What it parses
- Function declarations: `int foo(int a, float b);`
- Struct definitions: `struct Foo { int x; float y; };`
- Typedefs: `typedef int myint;`
- Constants: `#define MAX 100`

## Type mapping
| C type | Still type |
|--------|-----------|
| int, int32_t | int |
| long, int64_t | int |
| float | float |
| double | float |
| char* | str |
| void* | (opaque pointer, mapped to any) |
| bool, _Bool | bool |
| void | none |

## Limitations
- No macro expansion (only simple #define)
- No variadic functions (C's ...)
- No function pointers in params
- No nested structs
- No enum definitions (use Still enums)
- No #include resolution

## Implementation
The generator is a simple line-by-line parser that:
1. Strips comments
2. Identifies declarations
3. Maps C types to Still types
4. Emits `extern "C"` blocks