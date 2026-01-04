# Extern System Design

## Overview

`extern` is Lumen's explicit impurity boundary. It marks the point where Lumen's semantic guarantees end and host-defined behavior begins.

This document describes the design constraints that ensure the extern system remains **host-agnostic** and **language-independent**.

## Key Principle: Host Agnosticism

Lumen must not know about specific hosts (Rust, Python, JavaScript, etc.) or their implementation details.

This is enforced through **five design constraints**:

### 1. Host Ignorance

Lumen knows nothing about "Rust", "Python", or any specific host environment.

- Backend identifiers are opaque **strings**, not keywords or enumerated values
- Lumen never branches on backend identity
- No hardcoded assumptions about which backends exist
- No special-casing for particular hosts

**Implementation:** Backends are parsed as arbitrary identifiers in the selector string.

### 2. Capability-Based Semantics

`extern` requests **capabilities**, not implementations.

A capability is a named service (e.g., "print_native", "fs_open", "network_write") that the host may or may not provide.

**Example selectors:**
- `extern("print_native", value)` — request the default print capability
- `extern("fs:open", path)` — request the "open" capability from the "fs" backend
- `extern("fs|mem:read", key)` — try "fs" backend, then "mem" backend

Lumen never knows *how* these are implemented. It only knows they may succeed or fail.

### 3. Selector-as-Data

Selectors are **string values**, not syntax constructs.

- Parsed at runtime, not baked into language grammar
- No dots, no namespaces, no keywords
- Simple grammar: `"backend1|backend2|...:capability_name"`
- All identifiers treated uniformly (no special names)

**Implementation:** `selector::parse_selector()` is a pure function that tokenizes and validates the selector string.

### 4. Failure Honesty

If a selector explicitly names a backend, that backend **must** be available or the call fails.

- No silent fallback to other backends
- No auto-promotion to default implementations
- No guessing user intent

**Example:**
```lumen
extern("fs:open", path)   # MUST use fs; fail if unavailable
extern("open", path)      # TRY default; error if not found
```

### 5. Kernel Purity

The kernel must never know about capabilities or backends.

- Kernel provides value boxing and cloning only
- All host bindings live in adapter/registry layers
- Kernel treats all values opaquely via `RuntimeValue` trait
- No kernel code inspects what a capability is or does

**Implementation:**
- `Kernel::Value = Box<dyn RuntimeValue>` (opaque)
- `extern_system::call_extern(selector, args)` lives in the language, not kernel
- Capabilities are registered via `CapabilityRegistry`, external to kernel

## Architecture

### Components

**1. Selector Parser** (`selector.rs`)
- Input: A selector string
- Output: Ordered list of (backend, capability) clauses to try
- Property: Knows nothing about available backends or capabilities

**2. Capability Registry** (`registry.rs`)
- Stores `(backend_option, capability_name) → ExternCapability` mappings
- Trait-based design allows external adapters to register capabilities
- No hardcoded backends or capability names

**3. Capability Trait** (`registry.rs`)
```rust
pub trait ExternCapability: Send + Sync {
    fn name(&self) -> &'static str;
    fn call(&self, args: Vec<Value>) -> LumenResult<Value>;
}
```
- Each capability is a struct implementing this trait
- Responsible for its own validation and error handling
- Host adapters provide concrete implementations

**4. Call Dispatcher** (`mod.rs`)
```rust
pub fn call_extern(selector: &str, args: Vec<Value>) -> LumenResult<Value>
```
- Parses selector
- Resolves (backend, capability) in order
- Returns first match or error
- Travels opaquely through the evaluation pipeline

### Example: Adding a New Backend

To add support for a new host (e.g., Python):

1. Create a new module (outside Lumen): `python_adapter`
2. Implement capabilities as `ExternCapability`:
   ```rust
   struct PyWrite { ... }
   impl ExternCapability for PyWrite {
       fn name(&self) -> &'static str { "write" }
       fn call(&self, args: Vec<Value>) -> LumenResult<Value> { ... }
   }
   ```
3. Register with the registry:
   ```rust
   registry.register(Some("python"), Box::new(PyWrite))
   ```
4. Lumen code can now use: `extern("python:write", args...)`

**Lumen code does not change.** The selector is just a string.

## Phases

### Phase 1: Restore Foundations ✓
Implement strings as language-level values (not kernel assumptions).
- Result: `LumenString`, `LumenNumber`, `LumenBool` are language-specific
- Kernel does not know these types exist

### Phase 2: Kernel Refactor ✓
Replace `Value` enum with `RuntimeValue` trait.
- Result: Kernel is ontologically neutral
- Languages own value semantics and interpretation

### Phase 3: Shape Extern ✓
Design extern to be host-agnostic.
- Result: Selector grammar, registry, and capability trait are host-neutral
- No hardcoded backends, no special-casing of hosts

### Phase 4: External Adapters (Future)
Host environments provide `ExternCapability` implementations.
- Register capabilities via `CapabilityRegistry`
- Requires no changes to Lumen, kernel, or selector semantics

## Design Rationale

### Why Selector-as-Data?

If selectors were syntax-based (e.g., keywords for each backend), we would need to:
- Enumerate all possible backends at parse time
- Update the grammar when new backends are added
- Special-case each backend in the interpreter

Instead, selectors are strings. This means:
- New backends require no Lumen changes
- The language is stable across host environments
- Selector parsing is a simple, reusable algorithm

### Why Failure Honesty?

If we silently fell back from "fs:open" to a default implementation, programs would become fragile and hard to debug.

- A developer requests a specific backend for a reason
- Silently using something else violates the principle of least surprise
- Errors expose real problems (missing backends, misconfigured hosts)

### Why Not Keywords for Backends?

Example (bad design):
```lumen
extern_rust("fs_open", path)
extern_python("socket_connect", host)
```

This violates host agnosticism:
- Lumen knows about Rust and Python
- Adding a new host requires syntax changes
- Lumen becomes dependent on host ecosystem decisions

### Why Not Namespacing?

Example (bad design):
```lumen
extern("fs.open")    # fs is a namespace
extern("net.bind")   # net is a namespace
```

Namespaces imply semantic structure that Lumen shouldn't assume:
- Different hosts organize capabilities differently
- Lumen should treat "fs.open" and "open" identically
- The semantics of a colon vs. a dot belong to the host, not Lumen

## Current Capabilities (Built-in)

These are demonstration capabilities that show the pattern:

1. **print_native** — Print to stdout (impure)
   - Selector: `extern("print_native", value)`
   - Returns: The value that was printed

2. **debug_info** — Print diagnostic information (impure)
   - Selector: `extern("debug_info", value)`
   - Returns: The value passed in

3. **value_type** — Introspect value type
   - Selector: `extern("value_type", value)`
   - Returns: A number encoding the type (0=number, 1=bool, 2=string)

These are *minimal* and *language-specific*. They demonstrate that:
- Capabilities can access language-specific type information (via downcasting)
- The registry mechanism works
- The selector system is extensible

## Future Work

- [ ] Implement Python adapter (Python-specific capabilities)
- [ ] Implement filesystem adapter (fs:open, fs:read, fs:write)
- [ ] Implement network adapter (net:connect, net:send)
- [ ] Document how to write external adapters

## Guarantees

After this design is fully implemented, these invariants hold:

1. **Kernel purity:** Kernel code contains zero host-specific logic
2. **Language stability:** Lumen syntax and semantics never change to accommodate new hosts
3. **Extensibility:** New backends can be added without modifying Lumen, kernel, or selector logic
4. **Semantic clarity:** Selectors are data, not metadata; they travel through evaluation unchanged
