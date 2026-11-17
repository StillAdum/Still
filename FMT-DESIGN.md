# still fmt - Basic Formatter Design

## Overview
A code formatter for Still source files.
Normalizes indentation, spacing, and style.

## Rules
1. Indentation: 4 spaces per level
2. No trailing whitespace
3. Single blank line between top-level definitions
4. No blank lines at start/end of file
5. Consistent spacing around operators
6. Colon followed by space in type annotations: `x: int = 5`
7. No spaces inside brackets: `[1, 2, 3]` not `[ 1, 2, 3 ]`

## Implementation
- Parse the file with the Still lexer
- Re-emit with normalized formatting
- Preserve comments

## Status: Design complete, implementation pending