# Architectural Alignment: Restoring Foundations

## Executive Summary

The Lumen interpreter has been corrected to follow proper abstraction order:

1. **Kernel is ontologically neutral** — treats all values opaquely via `RuntimeValue` trait
2. **Strings exist as language-level values** — implemented as `LumenString`, not kernel assumptions
3. **Extern system is host-agnostic** — shaped for extensibility without hardcoded backends

This document explains *why* this order matters and how all three components align.

## The Sequencing Problem

The original development order was **incorrect**:
- extern was designed and partially scaffolded
- BEFORE strings were properly implemented as first-class values
- BEFORE the kernel was refactored to be ontologically neutral

This created dependencies in the wrong direction:
```
(WRONG ORDER)
extern → assumed concrete Value types → assumed string internals
```

The correct order is:
```
(CORRECT ORDER)
Kernel refactor → String foundation → Extern design
```

## Phase 1: Kernel Refactor (Completed)

### Problem
The kernel was baked with language-specific assumptions:
```rust
// OLD: Kernel knows about concrete types
pub enum Value {
    Number(String),
    Bool(bool),
    String(String),  // ← kernel makes assumptions about strings
}
```

This violated the principle that **the kernel should be ontologically neutral**.

### Solution
Replace with opaque trait abstraction:
```rust
// NEW: Kernel treats values as abstract
pub trait RuntimeValue: Send + Sync {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue>;
    fn as_debug_string(&self) -> String;
    fn as_display_string(&self) -> String;
    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String>;
    fn as_any(&self) -> &dyn Any;  // For type-safe downcasting
}

pub type Value = Box<dyn RuntimeValue>;
```

### Impact
- Kernel now contains zero language-specific logic
- Kernel mechanics are separated from language semantics
- Each language (Lumen, Mini-Rust, Mini-Python) defines its own value types

### Implementation
**Files created:**
- `src_lumen/values.rs` — LumenNumber, LumenBool, LumenString
- `src_mini_rust/values.rs` — MiniRustNumber, MiniRustBool
- `src_mini_python/values.rs` — MiniPythonNumber, MiniPythonBool

**Files updated:** All expression and statement evaluators
- Use language-specific constructors: `Box::new(LumenNumber::new(...))`
- Use language-specific helpers: `as_number()`, `as_bool()`, `as_string()`
- No pattern matching on concrete Value variants

## Phase 2: String Foundation (Completed)

### Problem
Before the kernel refactor, strings were partially baked into both kernel and language. This created fragile dependencies:
- The extern system could not reliably work with strings
- String semantics were scattered across kernel and language
- No clear ownership of string behavior

### Solution
After the kernel refactor, implement strings cleanly as a language value:

**`LumenString` (language-specific):**
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct LumenString {
    pub value: String,
}

impl RuntimeValue for LumenString {
    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_str) = other.as_any().downcast_ref::<LumenString>() {
            Ok(self.value == other_str.value)
        } else {
            Err("Cannot compare string with non-string".into())
        }
    }
    // ... other trait methods
}
```

**String is now:**
- Immutable (part of the type)
- First-class (passable through all expression evaluation)
- Comparable (== and !=)
- Printable (Display impl)
- Owned by the language, not the kernel

### Test Coverage
All 4 string examples pass:
- `string_basic.lm` — string literals and variables
- `string_comprehensive.lm` — equality, conditionals, mixed types
- `string_equality.lm` — == and !=
- `string_mixed.lm` — numbers and strings together

## Phase 3: Extern Host-Agnosticism (Completed)

### Problem
The extern system had been designed with implicit host assumptions:
- Selectors might become keywords (violating host agnosticism)
- Backends might be hardcoded (violating kernel purity)
- No clear abstraction layer for adapters

### Solution
Enforce strict host-agnosticism through **five design constraints**:

#### 1. Host Ignorance
Lumen knows nothing about "Rust", "Python", or any host.

**Evidence:**
- `selector.rs` — Parses backends as arbitrary alphanumeric strings
- No hardcoded list of valid backends
- No special-casing for any host

#### 2. Capability-Based Semantics
`extern` requests capabilities, not implementations.

**Example:**
```lumen
extern("print_native", value)      # Default print capability
extern("fs:open", path)            # fs backend, open capability
extern("fs|mem:read", key)         # Try fs then mem backend
```

The language never assumes which capabilities exist or how they're implemented.

#### 3. Selector-as-Data
Selectors are string values, not syntax constructs.

**Implementation:**
```rust
pub fn call_extern(selector: &str, args: Vec<Value>) -> LumenResult<Value> {
    let clauses = selector::parse_selector(selector)?;  // Parse at runtime
    // ... resolution happens in registry layer
}
```

No keywords, no special parsing, no compile-time assumptions.

#### 4. Failure Honesty
If a backend is explicitly named, it **must** be available or fail.

**Guarantee:**
- No silent fallback to different backends
- No auto-promotion to defaults
- Error message is clear about what was requested

#### 5. Kernel Purity
Kernel contains zero host logic.

**Evidence:**
- `src/kernel/runtime/value.rs` — No extern-specific code
- `src/kernel/` — No capability registry, no backend logic
- All host logic lives in `src_lumen/extern_system/` and external adapters

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Lumen Language Code                                         │
│ ─────────────────────────────────────────────────────────   │
│ extern("fs:open", path)  ← Selector is a string literal     │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Extern System (src_lumen/extern_system/)                    │
│ ─────────────────────────────────────────────────────────   │
│ 1. Parse selector string: "fs:open" → [(backend="fs", cap="open")]
│ 2. Resolve in registry: (fs, open) → ExternCapability     │
│ 3. Call capability with args: cap.call(args)              │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Capability Registry (external adapters)                     │
│ ─────────────────────────────────────────────────────────   │
│ [Potentially provided by host environment]                 │
│ - fs adapter: open, read, write, ...                       │
│ - net adapter: connect, send, recv, ...                    │
│ - db adapter: query, insert, ...                           │
└─────────────────────────────────────────────────────────────┘
```

### Design Rationale

**Why selector-as-data instead of keywords?**
- New backends require no Lumen changes
- No grammar updates needed
- Stable across host environments

**Why opaque backends instead of enums?**
- Lumen doesn't need to know about all possible backends
- Extensible without modifying language
- Follows principle of least commitment

**Why failure honesty?**
- Explicit requirements expose real problems
- Programs are easier to debug
- No invisible fallbacks create surprises

## Verification

### Build Status
```
cargo build
→ Finished `dev` profile [unoptimized + debuginfo]
```

### Test Results
```
bash test_all.sh
→ Total tests: 34
→ Passed: 34
→ Failed: 0
```

All tests pass across all three languages:
- Lumen (22 tests)
- Mini-Python (5 tests)
- Mini-Rust (5 tests)

### Test Categories
1. **Core functionality** (arithmetic, logic, control flow)
2. **Scoping and environments** (scope isolation, variable shadowing)
3. **Extern system** (print_native, debug_info, value_type)
4. **String support** (literals, comparison, mixed types)

## Alignment with Design Philosophy

The corrections align with Lumen's stated design principles:

| Principle | Implementation |
|-----------|-----------------|
| **Semantics first** | Kernel refactored to separate mechanics from meaning |
| **Explicit over clever** | Selectors are data, not hidden metadata |
| **Small, honest semantics** | No implicit backend assumptions |
| **Failure is a feature** | Missing capabilities produce clear errors |
| **Host ignorance** | Kernel and language make zero host assumptions |
| **Growth discipline** | One person can still understand entire system |

## Future Work (Phase 4)

The system is now ready for **external host adapters** to be registered without modifying Lumen or kernel code.

### Example: Adding a Filesystem Adapter

```rust
// In external adapter code (not Lumen)
struct FsOpen { /* ... */ }

impl ExternCapability for FsOpen {
    fn name(&self) -> &'static str { "open" }
    fn call(&self, args: Vec<Value>) -> LumenResult<Value> {
        // Implementation details...
    }
}

// Register with Lumen's registry
registry.register(Some("fs"), Box::new(FsOpen))
```

Lumen code unchanged:
```lumen
extern("fs:open", "/path/to/file")
```

### What Can Be Added
- Filesystem adapters (fs:open, fs:read, fs:write)
- Network adapters (net:connect, net:send)
- Database adapters (db:query, db:insert)
- Concurrency adapters (async:spawn, async:wait)
- Any capability expressible as `(backend, capability) → Value`

### What Cannot Change
- Kernel code (stays ontologically neutral)
- Lumen syntax (no new keywords)
- Selector grammar (no new delimiters)
- Value semantics (no new type assumptions)

## Summary

The work corrects a fundamental architectural error by restoring proper abstraction order:

1. **Kernel neutral** — Uses opaque RuntimeValue trait
2. **Strings founded** — Implemented as language-specific LumenString
3. **Extern shaped** — Host-agnostic design with five core constraints

This alignment ensures:
- ✓ Kernel remains stable and language-independent
- ✓ Languages can evolve without kernel changes
- ✓ New hosts can be added without modifying Lumen
- ✓ All 34 tests pass across all three languages
- ✓ System is ready for external adapters

The abstraction ladder is now properly ordered.
