// Quoted worths are written from the inside outward. The usual writer
// need not change how it has always shown an array or a map.

use crate::data::Value;
use num_traits::{Signed, ToPrimitive};
use num_bigint::BigInt;
use std::hash::{Hasher, Hash};

impl Value {
    pub fn kind_word(&self) -> String {
        let word = match self {
            Self::Complex(_) => "complex",
            Self::Thing(thing) => return thing.of.name.to_owned(),
            Self::Shared(cell) | Self::Mutable(cell, _) => return cell.borrow().kind_word(),
            Self::Tuple(_) | Self::Row(_) => "tuple", Self::Set(_) => "set", Self::Dict(_) => "dict",
            Self::Text(_) => "str", Self::Vector(_) => "list", Self::Flag(_) => "bool",
            Self::Small(_) | Self::Huge(_) => "int", Self::Frac(_) => "float",
            Self::Nil => "NoneType", Self::Progression(_) => "range", Self::Iterator(_) => "iterator",
            Self::Blueprint(_) | Self::KindOf(_) => "type", Self::Intrinsic(_) => "builtin_function_or_method",
            Self::Bound(..) | Self::Routine(_) => "function", _ => "object",
        };
        word.to_owned()
    }

    pub fn quoted(&self) -> String {
        fn surround(items: &[Value], left: &str, right: &str) -> String {
            let parts: Vec<_> = items.iter().map(Value::quoted).collect();
            format!("{}{}{}", left, parts.join(", "), right)
        }
        match self {
            Self::Nil => String::from("None"),
            Self::Flag(true) => String::from("True"), Self::Flag(false) => String::from("False"),
            Self::Vector(v) => surround(v, "[", "]"),
            Self::Tuple(v) | Self::Row(v) => surround(v, "(", if v.len() == 1 { ",)" } else { ")" }),
            Self::Set(v) => if v.is_empty() { String::from("set()") } else { surround(v, "{", "}") },
            Self::Dict(pairs) => {
                let mut rendered = Vec::new();
                for (key, value) in pairs.iter() { rendered.push(format!("{}: {}", key.quoted(), value.quoted())); }
                format!("{{{}}}", rendered.join(", "))
            }
            Self::Text(text) => {
                let mark = match (text.contains('\''), text.contains('"')) { (true, false) => '"', _ => '\'' };
                let mut quoted = mark.to_string();
                for letter in text.chars() {
                    if letter == mark || letter == '\\' { quoted.push('\\'); quoted.push(letter); continue; }
                    let escaped = match letter {
                        '\t' => String::from("\\t"), '\n' => String::from("\\n"), '\r' => String::from("\\r"),
                        c if c.is_control() || c.is_whitespace() && c != ' ' => match c as u32 {
                            n @ 0..=255 => format!("\\x{n:02x}"), n @ 256..=65535 => format!("\\u{n:04x}"), n => format!("\\U{n:08x}"),
                        },
                        c => c.to_string(),
                    };
                    quoted.push_str(&escaped);
                }
                quoted.push(mark);
                quoted
            }
            Self::Shared(cell) | Self::Mutable(cell, _) => cell.borrow().quoted(),
            Self::Frac(r) if r.places.is_some() => {
                if r.under && r.above == BigInt::from(0) { return String::from("-0.0"); }
                let f = crate::data::nearest_binary(&r.above, &r.beneath);
                match f { f if f.is_nan() => "nan".to_owned(), f if f.is_infinite() => if f.is_sign_negative() { "-inf" } else { "inf" }.to_owned(), f => format!("{f:?}") }
            }
            value => value.bare(),
        }
    }

    pub fn hash_number(&self) -> Option<i64> {
        let raw = match self {
            Self::Complex(pair) => {
                if pair.0.is_nan() || pair.1.is_nan() { return Some((std::rc::Rc::as_ptr(pair) as usize / 16) as i64); }
                let real = crate::complex::decimal_value(pair.0).hash_number()?;
                let imaginary = crate::complex::decimal_value(pair.1).hash_number()?;
                real.wrapping_add(1_000_003i64.wrapping_mul(imaginary))
            }
            Self::Text(chars) if chars.is_empty() => 0,
            Self::Text(chars) => {
                let mut state = std::collections::hash_map::DefaultHasher::new();
                chars.hash(&mut state);
                state.finish() as i64
            }
            Self::Nil => 0x9e3779b9,
            Self::Frac(parts) if parts.places.is_some() => {
                if parts.beneath == BigInt::from(0) {
                    return Some(if parts.above == BigInt::from(0) { (std::rc::Rc::as_ptr(parts) as usize / 16) as i64 } else if parts.above.is_negative() { -314159 } else { 314159 });
                }
                let prime = BigInt::from(2_305_843_009_213_693_951u64);
                let bottom = &parts.beneath % &prime;
                let positive = if bottom == BigInt::from(0) { 314159 }
                    else { ((parts.above.abs() % &prime) * bottom.modpow(&(&prime-2),&prime) % &prime).to_i64()? };
                if parts.above.is_negative() { -positive } else { positive }
            }
            Self::Tuple(parts) => {
                let mut accum: u64 = 2_870_177_450_012_600_261;
                for part in parts.iter() {
                    let lane = part.hash_number()? as u64;
                    accum = accum.wrapping_add(lane.wrapping_mul(14_029_467_366_897_019_727));
                    accum = accum.rotate_left(31);
                    accum = accum.wrapping_mul(11_400_714_785_074_694_791);
                }
                accum = accum.wrapping_add(parts.len() as u64 ^ (2_870_177_450_012_600_261u64 ^ 3_527_539));
                return Some(if accum == u64::MAX { 1_546_275_796 } else { accum as i64 });
            }
            Self::Huge(_) | Self::Small(_) | Self::Flag(_) => {
                let integer = self.as_big().ok()?;
                let residue = (integer.abs() % BigInt::from(2_305_843_009_213_693_951u64)).to_i64()?;
                if integer.is_negative() { -residue } else { residue }
            }
            _ => return None,
        };
        Some(if raw == -1 { -2 } else { raw })
    }
}
