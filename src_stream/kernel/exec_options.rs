// src_stream/kernel/exec_options.rs
//
// Execution options for the stream kernel.
// These options control optional optimizations and features.
//
// DESIGN PRINCIPLE: Optional features are OFF by default.
// No behavior changes unless explicitly enabled.

/// Execution options for the stream kernel.
///
/// These options control optional optimizations and behavioral features.
/// Default behavior is unchanged unless explicitly enabled.
#[derive(Debug, Clone)]
pub struct StreamExecutionOptions {
    /// Enable optional function-call memoization (caching of results).
    ///
    /// When enabled:
    /// - Only functions explicitly marked as memoizable are cached
    /// - Results are cached by (function_name, argument_values)
    /// - Recursive functions with memoization can execute exponentially faster
    ///
    /// When disabled (default):
    /// - No caching occurs
    /// - All function calls execute fully
    /// - Behavior is identical to stream kernel without this feature
    pub enable_memoization: bool,
}

impl StreamExecutionOptions {
    /// Create default execution options (all features disabled).
    pub fn new() -> Self {
        Self {
            enable_memoization: false,
        }
    }

    /// Create execution options with memoization enabled.
    pub fn with_memoization() -> Self {
        Self {
            enable_memoization: true,
        }
    }
}

impl Default for StreamExecutionOptions {
    fn default() -> Self {
        Self::new()
    }
}
