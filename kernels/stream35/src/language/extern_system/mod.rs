// Host capabilities are registered by backend and name and selected at runtime.
// Adapters implement ExternCapability and validate their own arguments.

pub mod capabilities;
pub mod registry;
pub mod selector;

use registry::CapabilityRegistry;
use std::sync::{Mutex, OnceLock};
use crate::kernel::runtime::Value;
use crate::kernel::registry::KernelResult as LumenResult;

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

