// src_lumen/extern_system/registry.rs
//
// Capability registry and trait definitions.
// Separates Lumen's semantic contracts from host implementations.

use crate::kernel::registry::LumenResult;
use crate::kernel::runtime::Value;
use std::collections::HashMap;

/// Trait defining a host capability implementation.
/// Each capability is responsible for:
/// - Validating its own arguments
/// - Performing the impure operation
/// - Returning a Lumen Value
pub trait ExternCapability: Send + Sync {
    /// Name of the capability (e.g., "print_native", "fs_open")
    fn name(&self) -> &'static str;

    /// Call the capability with the given arguments.
    /// Arguments are already evaluated Lumen values.
    /// Return a Value or a diagnostic error.
    fn call(&self, args: Vec<Value>) -> LumenResult<Value>;
}

/// Global capability registry.
/// Maps (backend_name_option, capability_name) pairs to implementations.
pub struct CapabilityRegistry {
    capabilities: HashMap<(Option<String>, String), Box<dyn ExternCapability>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    /// Register a capability with an optional backend.
    /// If backend is None, this is the default implementation.
    pub fn register(
        &mut self,
        backend: Option<&str>,
        cap: Box<dyn ExternCapability>,
    ) {
        let key = (backend.map(|s| s.to_string()), cap.name().to_string());
        self.capabilities.insert(key, cap);
    }

    /// Resolve a capability by (backend_option, capability_name).
    /// Returns the implementation if found, otherwise an error.
    pub fn resolve(
        &self,
        backend: &Option<String>,
        capability: &str,
    ) -> Option<&(dyn ExternCapability)> {
        self.capabilities
            .get(&(backend.clone(), capability.to_string()))
            .map(|b| b.as_ref())
    }

    /// Check if a capability is registered with an optional backend.
    pub fn has(&self, backend: &Option<String>, capability: &str) -> bool {
        self.capabilities
            .contains_key(&(backend.clone(), capability.to_string()))
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
