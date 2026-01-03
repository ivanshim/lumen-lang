// src_lumen/extern_system/selector.rs
//
// Selector parser for extern expressions.
//
// DESIGN PRINCIPLE: Selectors are opaque strings parsed at runtime.
// The grammar knows nothing about specific backends or hosts.
// All identifiers are treated as arbitrary strings.
// This ensures Lumen remains host-agnostic.
//
// Grammar:
//   selector ::= capability | backend ":" capability
//              | backend-list ":" capability
//   backend ::= word
//   backend-list ::= backend ( "|" backend )*
//                  | "(" backend-list ")"
//   capability ::= word
//
// Examples:
//   "print_native"     (capability only; no backend specified)
//   "fs:open"          (fs backend, open capability)
//   "fs|mem:read"      (try fs then mem backend, read capability)
//   "(fs:impl1)|(impl2)"  (complex fallback: fs:impl1 OR impl2)

use crate::src_stream::kernel::registry::LumenResult;

/// A selector clause: try to resolve (backend, capability) pair
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorClause {
    pub backend: Option<String>,
    pub capability: String,
}

/// Parse a selector string into ordered list of resolution clauses
/// Returns Vec of (backend_option, capability) pairs to try in order
pub fn parse_selector(input: &str) -> LumenResult<Vec<SelectorClause>> {
    let input = input.trim();

    if input.is_empty() {
        return Err("Empty selector".into());
    }

    // Try to find the rightmost ':' that's not inside parentheses
    let colon_pos = find_unparensed_colon(input);

    let (backend_part, capability_part) = if let Some(pos) = colon_pos {
        // Has backend specification
        (&input[..pos], &input[pos + 1..])
    } else {
        // No backend, just capability
        ("", input)
    };

    // Validate capability name (right of colon)
    if !is_valid_name(capability_part) {
        return Err(format!("Invalid capability name: '{}'", capability_part));
    }

    let capability = capability_part.to_string();

    if backend_part.is_empty() {
        // No backend specified - try default
        Ok(vec![SelectorClause {
            backend: None,
            capability,
        }])
    } else {
        // Parse backend list
        let backends = parse_backend_list(backend_part)?;

        Ok(backends
            .into_iter()
            .map(|backend| SelectorClause {
                backend: Some(backend),
                capability: capability.clone(),
            })
            .collect())
    }
}

/// Find the rightmost ':' that is not inside parentheses
fn find_unparensed_colon(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in s.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => depth -= 1,
            ':' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Parse a backend list: "fs|mem" or "(fs|mem)" or complex nesting
fn parse_backend_list(input: &str) -> LumenResult<Vec<String>> {
    let input = input.trim();

    if input.is_empty() {
        return Err("Empty backend list".into());
    }

    // Handle parentheses
    if input.starts_with('(') && input.ends_with(')') {
        let inner = &input[1..input.len() - 1];
        return parse_backend_list(inner);
    }

    // Split by '|' at top level (not inside parens)
    let mut backends = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            '|' if depth == 0 => {
                let backend = current.trim().to_string();
                if !backend.is_empty() {
                    backends.push(backend);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let backend = current.trim().to_string();
    if !backend.is_empty() {
        backends.push(backend);
    }

    if backends.is_empty() {
        return Err("No backends in backend list".into());
    }

    // Validate all backends and unwrap parentheses if needed
    backends = backends
        .into_iter()
        .map(|b| {
            let b = b.trim();
            if b.starts_with('(') && b.ends_with(')') {
                b[1..b.len() - 1].to_string()
            } else {
                b.to_string()
            }
        })
        .collect();

    for backend in &backends {
        if !is_valid_name(backend) {
            return Err(format!("Invalid backend name: '{}'", backend));
        }
    }

    Ok(backends)
}

/// Check if a string is a valid identifier (word)
fn is_valid_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    s.chars()
        .all(|c| c.is_alphanumeric() || c == '_')
        && s.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_capability() {
        let result = parse_selector("print_native").unwrap();
        assert_eq!(
            result,
            vec![SelectorClause {
                backend: None,
                capability: "print_native".into()
            }]
        );
    }

    #[test]
    fn test_parse_backend_colon_capability() {
        let result = parse_selector("fs:open").unwrap();
        assert_eq!(
            result,
            vec![SelectorClause {
                backend: Some("fs".into()),
                capability: "open".into()
            }]
        );
    }

    #[test]
    fn test_parse_backend_list() {
        let result = parse_selector("fs|mem:read").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].backend, Some("fs".into()));
        assert_eq!(result[1].backend, Some("mem".into()));
        assert_eq!(result[0].capability, "read");
        assert_eq!(result[1].capability, "read");
    }

    #[test]
    fn test_parse_complex() {
        let result = parse_selector("(fs:impl1)|(impl2)").unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_invalid_empty() {
        assert!(parse_selector("").is_err());
    }

    #[test]
    fn test_invalid_bad_backend() {
        assert!(parse_selector("123bad:open").is_err());
    }
}
