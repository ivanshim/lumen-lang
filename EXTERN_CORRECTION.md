# Extern System Correction: String Literal Enforcement

## Problem

The `extern` system had drifted from its design intent:

**Design Intent:** Selectors are **opaque data strings**, not identifiers.
- Selectors must be quoted: `extern("print_native", args...)`
- Selectors are parsed at runtime as arbitrary alphanumeric strings
- Lumen makes no assumptions about selector format or meaning

**Implementation Drift:** The parser was accepting unquoted identifiers.
- Selectors could be written: `extern(print_native, args...)`
- This created implicit identifier-to-string coercion
- This violated the principle that selectors are data, not syntax

## Impact

The drift had two consequences:

1. **Design Violation:** Lumen appeared to know about capability names (print_native, debug_info, value_type)
2. **Coercion:** Unquoted identifiers were silently converted to strings, hiding the boundary between syntax and data

## Solution

### 1. Parser Enforcement (extern_expr.rs)

Changed the parser to:
- **Require** selectors to be string literals (must start with `"`)
- **Reject** any non-string selector with a clear error message
- **Extract** the selector content by removing quotes

**Error message:**
```
extern selector must be a string literal (e.g., "print_native").
Selector is data, not an identifier.
Use: extern("capability", args...)
Not: extern(capability, args...)
```

### 2. Example Updates

Updated all 9 extern example files:
- `extern_args.lm`
- `extern_basic.lm`
- `extern_comprehensive.lm`
- `extern_debug.lm`
- `extern_fallback.lm`
- `extern_multiarg.lm`
- `extern_simple.lm`
- `extern_test.lm`
- `extern_type.lm`

Changed:
```lumen
# OLD (incorrect)
extern(print_native, 42)
extern(debug_info, value)
extern(value_type, 100)

# NEW (correct)
extern("print_native", 42)
extern("debug_info", value)
extern("value_type", 100)
```

## Design Rationale

### Why String Literals?

1. **Data vs Syntax:** Strings make it explicit that selectors are data, not language syntax
2. **Host Agnosticism:** Prevents Lumen from appearing to know about capability names
3. **Runtime Parsing:** Selectors travel opaquely through the evaluation pipeline
4. **No Coercion:** Eliminates implicit type conversion at the semantic boundary

### Why Clear Errors?

1. **Honesty:** Users see exactly what went wrong
2. **Guidance:** The error message explains the correct syntax
3. **Clarity:** Leaves no doubt about what Lumen expects

## Verification

### Tests
- ✅ All 34 tests pass
- ✅ All 11 extern examples execute correctly
- ✅ Build succeeds with no errors

### Error Handling
- ✅ Invalid syntax (unquoted identifiers) produces helpful error
- ✅ Valid syntax (quoted strings) executes correctly
- ✅ Error message guides users to correct usage

### Examples
**Valid (accepted):**
```lumen
extern("print_native", 42)
extern("debug_info", value)
extern("value_type", 100)
extern("fs:open", "/path")
extern("fs|mem:read", key)
```

**Invalid (rejected with error):**
```lumen
extern(print_native, 42)          # Error: not a string literal
extern(debug_info, value)         # Error: not a string literal
x = print_native                  # Error: identifier, not string
```

## Alignment with Design Principles

This correction aligns with Lumen's design principles:

| Principle | Evidence |
|-----------|----------|
| **Data as data** | Selectors are now clearly strings, not identifiers |
| **Explicit over implicit** | No hidden identifier-to-string coercion |
| **Honesty in error** | Clear message when syntax is wrong |
| **Host ignorance** | Lumen doesn't know what "print_native" means |
| **Semantic clarity** | Selector is opaque data, travels unchanged |

## Conclusion

The extern system is now properly aligned with its design intent:

1. **Selectors are strings** — explicitly quoted, never identifiers
2. **No host knowledge** — Lumen makes no assumptions about selector names
3. **Clear boundaries** — Error messages guide users to correct usage
4. **Kernel pure** — No host logic in kernel code
5. **Runtime parsing** — Selectors are opaque data at compile time

The system is ready for external host adapters to register capabilities without modifying Lumen code.
