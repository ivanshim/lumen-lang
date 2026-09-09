// The small values used by the collection builtins. Quoting descends
// through a collection, while the ordinary writer keeps its old form.

use crate::value::Value;
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
use std::hash::{Hash, Hasher};

impl Value {
    pub fn core_kind(&self) -> String {
        match self {
            Value::Small(_) | Value::Huge(_) => "int",
            Value::Real(_) | Value::Frac(_) => "float",
            Value::Text(_) => "str",
            Value::Flag(_) => "bool",
            Value::Null => "NoneType",
            Value::Array(_) => "list",
            Value::Tuple(_) => "tuple",
            Value::Set(_) => "set",
            Value::Map(_) => "dict",
            Value::Counted(_) => "range",
            Value::Cursor(_) => "iterator",
            Value::Native(..) => "builtin_function_or_method",
            Value::Routine(_) => "function",
            Value::Class(_) | Value::SortOf(_) => "type",
            Value::Object(o) => return o.class.name.clone(),
            Value::Bond(c) => return c.borrow().core_kind(),
            _ => "object",
        }.to_string()
    }

    pub fn core_repr(&self) -> String {
        match self {
            Value::Text(s) => {
                let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
                let mut out = String::from(quote);
                for c in s.chars() {
                    match c {
                        '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"),
                        '\\' => out.push_str("\\\\"),
                        c if c == quote => { out.push('\\'); out.push(c); }
                        c if c.is_control() || c == '\u{a0}' => {
                            let n = c as u32;
                            out.push_str(&if n < 256 { format!("\\x{:02x}", n) } else if n < 65536 { format!("\\u{:04x}", n) } else { format!("\\U{:08x}", n) });
                        }
                        c => out.push(c),
                    }
                }
                out.push(quote);
                out
            }
            Value::Array(a) => format!("[{}]", a.iter().map(Value::core_repr).collect::<Vec<_>>().join(", ")),
            Value::Tuple(a) => format!("({}{})", a.iter().map(Value::core_repr).collect::<Vec<_>>().join(", "), if a.len() == 1 { "," } else { "" }),
            Value::Set(a) if a.is_empty() => "set()".into(),
            Value::Set(a) => format!("{{{}}}", a.iter().map(Value::core_repr).collect::<Vec<_>>().join(", ")),
            Value::Map(a) => format!("{{{}}}", a.iter().map(|(k,v)| format!("{}: {}", k.core_repr(), v.core_repr())).collect::<Vec<_>>().join(", ")),
            Value::Null => "None".into(),
            Value::Flag(b) => if *b { "True" } else { "False" }.into(),
            Value::Bond(c) => c.borrow().core_repr(),
            Value::Real(r) => {
                let number = crate::value::as_binary(&r.p, &r.q);
                if number.is_nan() { "nan".into() } else if number == f64::INFINITY { "inf".into() }
                else if number == f64::NEG_INFINITY { "-inf".into() } else { format!("{:?}", number) }
            }
            other => other.plain(),
        }
    }

    pub fn core_hash(&self) -> Option<i64> {
        let finish = |n| if n == -1 { -2 } else { n };
        match self {
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) => {
                let number = self.as_big().ok()?;
                let modulus = BigInt::from((1u64 << 61) - 1);
                let mut h = (number.abs() % modulus).to_i64()?;
                if number.is_negative() { h = -h; }
                Some(finish(h))
            }
            Value::Text(s) => {
                if s.is_empty() { return Some(0); }
                let mut h = std::collections::hash_map::DefaultHasher::new();
                s.hash(&mut h);
                Some(finish(h.finish() as i64))
            }
            Value::Null => Some(0x9e3779b9),
            Value::Real(r) => {
                if r.q == BigInt::from(0) { return if r.p == BigInt::from(0) { Some((std::rc::Rc::as_ptr(r) as usize >> 4) as i64) } else { Some(if r.p.is_negative() { -314159 } else { 314159 }) }; }
                let modulus = BigInt::from((1u64 << 61)-1);
                let denom = &r.q % &modulus;
                if denom == BigInt::from(0) { return Some(if r.p.is_negative() { -314159 } else { 314159 }); }
                let inverse = denom.modpow(&(&modulus-2), &modulus);
                let mut h = ((r.p.abs() % &modulus) * inverse % &modulus).to_i64()?;
                if r.p.is_negative() { h = -h; }
                Some(finish(h))
            }
            Value::Tuple(items) => {
                let mut h = 2870177450012600261u64;
                for item in items.iter() {
                    h = h.wrapping_add((item.core_hash()? as u64).wrapping_mul(14029467366897019727));
                    h = h.rotate_left(31).wrapping_mul(11400714785074694791);
                }
                h = h.wrapping_add(items.len() as u64 ^ (2870177450012600261 ^ 3527539));
                Some(if h == u64::MAX { 1546275796 } else { h as i64 })
            }
            _ => None,
        }
    }
}
