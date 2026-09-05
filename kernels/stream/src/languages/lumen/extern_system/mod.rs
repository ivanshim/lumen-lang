// src_lumen/extern_system/mod.rs
//
// The extern system: Lumen's explicit impurity boundary.
//
// extern is a reserved keyword that marks where Lumen's semantic guarantees stop
// and host-defined behavior begins. It is deliberately uncomfortable to use,
// making the boundary explicit in source code.
//
// DESIGN STATUS: extern is correctly shaped and ready for host adapters.
// Kernel and language layers are host-agnostic. External implementations may
// register capabilities without modifying Lumen or kernel code.
//
// =============================================================================
// HOST-AGNOSTICISM CONSTRAINTS
// =============================================================================
//
// These constraints ensure that Lumen remains ontologically neutral regarding
// the host environment:
//
// 1. HOST IGNORANCE
//    - Lumen knows nothing about "Rust", "Python", or any specific host
//    - Backend identifiers are opaque strings (selector, not keywords)
//    - Lumen never branches on backend identity
//    - Selectors are parsed at runtime, not baked into syntax
//
// 2. CAPABILITY-BASED SEMANTICS
//    - extern requests *capabilities*, not implementations
//    - Example selectors:
//      - "print_native"        (no backend: default implementation)
//      - "fs:open"             (backend fs: capability open)
//      - "fs|mem:read"         (multiple backends: try in order)
//    - The selector grammar is in selector.rs; it is host-agnostic
//
// 3. SELECTOR-AS-DATA
//    - Selectors are string values
//    - Parsed at runtime by selector::parse_selector()
//    - No dots, no namespaces, no keywords
//    - Syntax: "backend1|backend2:capability_name"
//
// 4. FAILURE HONESTY
//    - If an explicitly requested backend is unavailable → error (no fallback)
//    - No silent backend guessing
//    - No auto-promotion to different backends
//
// 5. KERNEL PURITY
//    - Kernel provides execution mechanics only (value boxing, cloning, etc.)
//    - All host bindings live in adapter/registry layers
//    - Kernel does not know about capabilities or backends
//    - Kernel treats all values opaquely (RuntimeValue trait)
//
// =============================================================================
// IMPLEMENTATION PHASES
// =============================================================================
//
// Phase 1 (✓ COMPLETE): Restore foundations
//   - Implement strings as language-level values (not kernel assumptions)
//   - Result: LumenString, LumenNumber, LumenBool are language-specific
//
// Phase 2 (✓ COMPLETE): Kernel refactor
//   - Replace Value enum with RuntimeValue trait in kernel
//   - Result: Kernel is ontologically neutral; languages own value semantics
//
// Phase 3 (✓ COMPLETE): Shape extern
//   - Design extern to be host-agnostic (no hardcoded backends)
//   - Result: Selector grammar, registry, and capability trait are host-neutral
//
// Phase 4 (PENDING): External adapters
//   - Host environments provide ExternCapability implementations
//   - Register capabilities via CapabilityRegistry
//   - Requires no changes to Lumen, kernel, or selector semantics
//   - Example: A Rust adapter might implement "fs:open", "net:connect", etc.
//
// =============================================================================
// ADDING NEW CAPABILITIES
// =============================================================================
//
// To extend extern without modifying Lumen:
//
// 1. Create a struct implementing ExternCapability trait (registry.rs)
// 2. Implement the ExternCapability::call() method
// 3. Register via CapabilityRegistry::register(backend, capability)
// 4. Invoke from Lumen: extern("backend:capability", args...)
//
// The kernel and language remain unchanged.
// The selector string travels opaquely through the evaluation pipeline.
// Host adapters are responsible for their own validation, error handling,
// and argument interpretation.
//
// =============================================================================

pub mod capabilities;
pub mod registry;
pub mod selector;

use registry::CapabilityRegistry;
use std::sync::{Mutex, OnceLock};
use crate::kernel::runtime::Value;
use crate::kernel::registry::LumenResult;

/// Global capability registry (lazily initialized)
fn get_registry() -> &'static Mutex<CapabilityRegistry> {
    static REGISTRY: OnceLock<Mutex<CapabilityRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = CapabilityRegistry::new();
        capabilities::register_builtins(&mut registry);
        Mutex::new(registry)
    })
}

/// Call an extern capability with the given selector and arguments.
/// This is the boundary crossing function.
pub fn call_extern(
    selector: &str,
    args: Vec<Value>,
) -> LumenResult<Value> {
    // Parse the selector string
    let clauses = selector::parse_selector(selector)?;

    // Resolve the capability in order
    let registry = get_registry();
    let registry = registry.lock().unwrap();

    for clause in &clauses {
        if let Some(cap) = registry.resolve(&clause.backend, &clause.capability) {
            // Found a matching capability - call it
            return cap.call(args);
        }
    }

    // No capability found in any clause
    let first_clause = clauses.first().ok_or_else(|| "Empty selector clauses".to_string())?;
    Err(format!(
        "No implementation found for capability '{}' with backends {:?}",
        first_clause.capability,
        clauses.iter().filter_map(|c| c.backend.as_ref()).collect::<Vec<_>>()
    ))
}

