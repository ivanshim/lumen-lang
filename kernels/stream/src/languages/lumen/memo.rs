// MEMOIZATION: Lumen's function-result cache.
//
// `MEMOIZATION = true` enables caching for every call made while the binding
// is in effect. The flag is an ordinary scoped binding, so callees inherit it
// through the scope chain and it is restored automatically when the scope
// that set it is left. The cache itself lives in the environment's extension
// slot; the kernel knows nothing about either.

use std::collections::HashMap;

use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::values::LumenBool;

/// The switch's name: a reserved word, so programs cannot read it as a value.
fn switch() -> &'static str {
    crate::languages::lumen::definition::def().first("system.memoization")
}

#[derive(Default)]
pub struct MemoCache {
    entries: HashMap<(String, String), Value>,
}

pub fn set_enabled(env: &mut Env, enabled: bool) {
    env.define(switch().to_string(), Box::new(LumenBool::new(enabled)));
}

pub fn enabled(env: &Env) -> bool {
    env.get(switch())
        .ok()
        .and_then(|v| v.as_any().downcast_ref::<LumenBool>().map(|b| b.value))
        .unwrap_or(false)
}

fn key(func_name: &str, args: &[Value]) -> (String, String) {
    let fingerprint = args.iter().map(|v| v.as_debug_string()).collect::<Vec<_>>().join("|");
    (func_name.to_string(), fingerprint)
}

/// Cached result for this call, if memoization is on and the call was seen.
pub fn lookup(env: &mut Env, func_name: &str, args: &[Value]) -> Option<Value> {
    if !enabled(env) {
        return None;
    }
    let k = key(func_name, args);
    env.extension::<MemoCache>().entries.get(&k).cloned()
}

/// Remember the result of this call, if memoization is on.
pub fn store(env: &mut Env, func_name: &str, args: &[Value], result: &Value) {
    if !enabled(env) {
        return;
    }
    let k = key(func_name, args);
    env.extension::<MemoCache>().entries.insert(k, result.clone());
}
