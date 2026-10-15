// Serialized forms preserve compiled calls and bindings independently of source availability.
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde_json::{Value as Node, json};
use std::{rc::Rc, cell::RefCell};
use crate::data::Value;

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, output: S) -> Result<S::Ok, S::Error> {
        let tagged = match self {
            Value::Nil => json!([0, null]),
            Value::Ellipsis => json!([1, null]),
            Value::Unset => json!([2, null]),
            Value::OctetKind { changeable, shown } => json!([20, [changeable, shown]]),
            Value::Channel(channel) => json!([21, channel]),
            Value::Flag(flag) => json!([3, flag]),
            Value::Small(integer) => json!([4, integer]),
            Value::Huge(integer) => json!([5, integer.to_string()]),
            Value::Text(word) => json!([6, word]),
            Value::Unpaired(characters) => json!([7, characters]),
            Value::Octets { cell, changeable: false, lead } => json!([8, [cell.borrow().as_slice(), lead]]),
            Value::Tuple(row) => json!([9, serde_json::to_value(row.as_slice()).map_err(serde::ser::Error::custom)?]),
            Value::Vector(row) => json!([10, serde_json::to_value(row.as_slice()).map_err(serde::ser::Error::custom)?]),
            Value::Routine(routine) => json!([11, record_body(routine).map_err(serde::ser::Error::custom)?]),
            Value::Intrinsic(work, spelling) => json!([12, [work, spelling]]),
            Value::Frac(ratio) => json!([13, ratio]),
            Value::Imaginary { coefficient, unready } => json!([14, [coefficient.to_bits(), unready]]),
            Value::Wrapped(mark, row) => json!([16, serde_json::to_value((mark, row.as_slice())).map_err(serde::ser::Error::custom)?]),
            Value::Complex(parts) => json!([17, [parts.0.to_bits(), parts.1.to_bits(), parts.2]]),
            Value::KindOf(kind) => json!([18, kind]),
            Value::TextRow(words, marked) => json!([19, [words, marked]]),
            Value::Refusal(spelling) => json!([15, spelling]),
            _ => return Err(serde::ser::Error::custom(format!("compiled form cannot retain {}", self.kind_word()))),
        };
        tagged.serialize(output)
    }
}
impl<'a> Deserialize<'a> for Value {
    fn deserialize<D: Deserializer<'a>>(input: D) -> Result<Self, D::Error> {
        let mut parts = Vec::<Node>::deserialize(input)?;
        if parts.len() != 2 { return Err(serde::de::Error::custom("invalid constant record")); }
        let payload = parts.pop().unwrap();
        let discriminator = parts[0].as_u64().ok_or_else(|| serde::de::Error::custom("invalid constant tag"))?;
        macro_rules! recover { ($kind:ty) => { serde_json::from_value::<$kind>(payload).map_err(serde::de::Error::custom)? }; }
        let value = match discriminator {
            0 => Value::Nil,
            1 => Value::Ellipsis,
            2 => Value::Unset,
            20 => { let (changeable, shown) = recover!((bool, Rc<str>)); Value::OctetKind { changeable, shown } },
            21 => Value::Channel(recover!(u8)),
            3 => Value::Flag(recover!(bool)),
            4 => Value::Small(recover!(i64)),
            5 => Value::Huge(Rc::new(recover!(String).parse().map_err(serde::de::Error::custom)?)),
            6 => Value::Text(recover!(Rc<str>)),
            7 => Value::Unpaired(Rc::from(recover!(Vec<u32>))),
            8 => {
                let (octets, lead) = recover!((Vec<u8>, Rc<str>));
                Value::Octets { cell: Rc::new(RefCell::new(octets)), changeable: false, lead }
            },
            9 => Value::tuple(recover!(Vec<Value>)),
            10 => Value::Vector(crate::tuples::Sequence::plain(recover!(Vec<Value>))),
            11 => Value::Routine(body_from_record(payload).map_err(serde::de::Error::custom)?),
            12 => { let (primitive, word) = recover!((crate::form::Prim, Rc<str>)); Value::Intrinsic(primitive, word) },
            13 => Value::Frac(Rc::new(recover!(crate::data::Ratio))),
            14 => { let (bits, unready) = recover!((u64, Rc<str>)); Value::Imaginary { coefficient: f64::from_bits(bits), unready } },
            16 => { let (mark, values) = recover!((u8, Vec<Value>)); Value::Wrapped(mark, crate::tuples::Sequence::plain(values)) },
            17 => { let (real, imag, words) = recover!((u64, u64, Rc<str>)); Value::Complex(Rc::new((f64::from_bits(real), f64::from_bits(imag), words))) },
            18 => Value::KindOf(recover!(crate::data::Kind)),
            19 => { let (words, marked) = recover!((Rc<Vec<String>>, bool)); Value::TextRow(words, marked) },
            15 => Value::Refusal(recover!(Rc<str>)),
            _ => return Err(serde::de::Error::custom("unknown constant tag")),
        };
        Ok(value)
    }
}

type Namespace = (Option<Value>, Option<Value>, Option<Rc<str>>, Vec<Rc<Vec<crate::scan::Token>>>);
thread_local! { static READ_NAMESPACES: RefCell<Vec<Namespace>> = const { RefCell::new(Vec::new()) }; }
pub fn outer_dictionary() -> Option<Value> { READ_NAMESPACES.with(|stack| stack.borrow().last().and_then(|names| names.0.clone())) }
pub fn native_dictionary() -> Option<Value> { READ_NAMESPACES.with(|stack| stack.borrow().last().and_then(|names| names.1.clone())) }
pub fn namespace_name() -> Option<Rc<str>> { READ_NAMESPACES.with(|stack| stack.borrow().last().and_then(|names| names.2.clone())) }
struct ReadingScope;
impl Drop for ReadingScope {
    fn drop(&mut self) { READ_NAMESPACES.with(|stack| { stack.borrow_mut().pop(); }); BODY_INPUT.with(|stack| { stack.borrow_mut().pop(); }); }
}

pub fn forms(mut record: Node, seeded: &[String], globals: Option<Value>, natives: Option<Value>, module: Option<Rc<str>>) -> Result<crate::build::Built, String> {
    if record["format"] != "lumen-microcode7-v6" { return Err("ImportError: compiled forms use a different ABI".to_owned()); }
    let mut names = seeded.to_vec();
    let prior: Vec<String> = serde_json::from_value(record["metadata"]["globals"].clone()).map_err(|error| format!("ValueError: {error}"))?;
    let mut offsets = Vec::with_capacity(prior.len());
    for name in prior {
        let location = match names.iter().position(|present| present == &name) {
            Some(found) => found,
            None => { names.push(name); names.len() - 1 }
        };
        offsets.push(location);
    }
    fn renumber(node: &mut Node, offsets: &[usize]) -> Result<(), String> {
        if let Some(attributes) = node.as_object_mut() {
            if attributes.contains_key("cache_global") {
                let address = attributes["at"].as_u64().ok_or("ValueError: malformed address")? as usize;
                if attributes["cache_global"] == true {
                    attributes.insert("at".into(), json!(*offsets.get(address).ok_or("ValueError: global slot outside cache")?));
                }
                if let Some(old) = attributes["fallback"].as_u64() {
                    attributes.insert("fallback".into(), json!(*offsets.get(old as usize).ok_or("ValueError: invalid global fallback")?));
                }
            }
            for child in attributes.values_mut() { renumber(child, offsets)?; }
        } else if let Some(children) = node.as_array_mut() {
            for child in children { renumber(child, offsets)?; }
        }
        Ok(())
    }
    renumber(&mut record["forms"], &offsets)?;
    renumber(&mut record["bodies"], &offsets)?;
    let spellings = serde_json::from_value(record["spellings"].clone()).map_err(|error| format!("ValueError: {error}"))?;
    let bodies: Vec<Node> = serde_json::from_value(record["bodies"].take()).map_err(|e| format!("ValueError: {e}"))?;
    BODY_INPUT.with(|stack| stack.borrow_mut().push(BodyInput { visited: vec![None; bodies.len()], pending: Default::default(), bodies }));
    READ_NAMESPACES.with(|stack| stack.borrow_mut().push((globals, natives, module, spellings)));
    let _reading = ReadingScope;
    let program = serde_json::from_value(record["forms"].take()).map_err(|error| format!("ValueError: {error}"))?;
    crate::build::cached_forms(program, names, record["metadata"].take())
}

struct Spellings {
    rows: Vec<Rc<Vec<crate::scan::Token>>>,
    lookup: std::collections::HashMap<usize, usize>,
}
thread_local! { static OUTPUT_WORDS: RefCell<Option<Spellings>> = const { RefCell::new(None) }; }
pub fn save_spelling<S: Serializer>(word: &Rc<Vec<crate::scan::Token>>, out: S) -> Result<S::Ok, S::Error> {
    let reference = OUTPUT_WORDS.with(|cell| {
        let mut state = cell.borrow_mut();
        let table = state.as_mut()?;
        let identity = Rc::as_ptr(word) as usize;
        if let Some(existing) = table.lookup.get(&identity) { return Some(*existing); }
        let position = table.rows.len();
        table.rows.push(word.clone());
        table.lookup.insert(identity, position);
        Some(position)
    });
    if let Some(reference) = reference { reference.serialize(out) } else { word.serialize(out) }
}
pub fn recover_spelling<'de, D: Deserializer<'de>>(from: D) -> Result<Rc<Vec<crate::scan::Token>>, D::Error> {
    match Node::deserialize(from)? {
        Node::Array(parts) => serde_json::from_value::<Vec<crate::scan::Token>>(Node::Array(parts)).map(Rc::new).map_err(serde::de::Error::custom),
        Node::Number(number) => {
            let at = number.as_u64().ok_or_else(|| serde::de::Error::custom("bad source symbol"))?;
            READ_NAMESPACES.with(|scope| scope.borrow().last().and_then(|environment| environment.3.get(at as usize).cloned()))
                .ok_or_else(|| serde::de::Error::custom("missing source spelling"))
        },
        _ => Err(serde::de::Error::custom("source metadata has wrong type")),
    }
}
pub fn serialize_forms(routine: Rc<crate::form::Routine>) -> Result<(Node, Vec<Rc<Vec<crate::scan::Token>>>, Vec<Node>), String> {
    let previous = OUTPUT_WORDS.with(|cell| cell.replace(Some(Spellings { rows: Vec::new(), lookup: std::collections::HashMap::new() })));
    let previous_bodies = BODY_OUTPUT.with(|slot| slot.replace(Some(BodyOutput::default())));
    let result = serde_json::to_value(routine);
    let bodies = BODY_OUTPUT.with(|slot| slot.replace(previous_bodies).unwrap().nodes);
    let entries = OUTPUT_WORDS.with(|cell| cell.replace(previous).unwrap().rows);
    result.map(|forms| (forms, entries, bodies)).map_err(|error| format!("ValueError: {error}"))
}


pub fn relocate_source(value: &mut Node, previous: &str, replacement: &str) {
    if let Some(row) = value.as_array_mut() {
        row.iter_mut().for_each(|entry| relocate_source(entry, previous, replacement));
    } else if let Some(object) = value.as_object_mut() {
        for (key, content) in object {
            if key == "written_in" && content.as_str() == Some(previous) {
                *content = Node::String(replacement.to_owned());
            } else {
                relocate_source(content, previous, replacement);
            }
        }
    }
}


#[derive(Default)]
struct BodyOutput { nodes: Vec<Node>, positions: std::collections::HashMap<usize, usize> }
struct BodyInput {
    bodies: Vec<Node>,
    visited: Vec<Option<Rc<crate::form::Routine>>>,
    pending: std::collections::BTreeSet<usize>,
}
thread_local! {
    static BODY_OUTPUT: RefCell<Option<BodyOutput>> = const { RefCell::new(None) };
    static BODY_INPUT: RefCell<Vec<BodyInput>> = const { RefCell::new(Vec::new()) };
}
fn record_body(routine: &Rc<crate::form::Routine>) -> Result<Node, String> {
    let pointer = Rc::as_ptr(routine) as usize;
    let location = BODY_OUTPUT.with(|output| {
        let mut output = output.borrow_mut();
        let state = output.as_mut()?;
        match state.positions.get(&pointer).copied() {
            Some(at) => Some((at, true)),
            None => {
                let at = state.nodes.len();
                state.nodes.push(Node::Null);
                state.positions.insert(pointer, at);
                Some((at, false))
            }
        }
    });
    match location {
        None => serde_json::to_value(routine).map_err(|err| err.to_string()),
        Some((at, found)) => {
            if !found {
                let body = serde_json::to_value(routine).map_err(|err| err.to_string())?;
                BODY_OUTPUT.with(|output| output.borrow_mut().as_mut().unwrap().nodes[at] = body);
            }
            Ok(Node::from(at))
        }
    }
}
fn body_from_record(value: Node) -> Result<Rc<crate::form::Routine>, String> {
    if !value.is_number() { return serde_json::from_value(value).map_err(|err| err.to_string()); }
    let position = value.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("invalid compiled body reference")?;
    let saved = BODY_INPUT.with(|input| {
        let mut input = input.borrow_mut();
        let scope = input.last_mut().ok_or("no compiled body table")?;
        if let Some(Some(existing)) = scope.visited.get(position) { return Ok((Some(existing.clone()), Node::Null)); }
        let body = scope.bodies.get(position).ok_or("compiled body reference out of range")?.clone();
        if !scope.pending.insert(position) { return Err("recursive compiled body reference"); }
        Ok((None, body))
    })?;
    if let Some(existing) = saved.0 { return Ok(existing); }
    let result = serde_json::from_value::<Rc<crate::form::Routine>>(saved.1).map_err(|err| err.to_string());
    BODY_INPUT.with(|input| {
        let mut input = input.borrow_mut();
        let scope = input.last_mut().unwrap();
        scope.pending.remove(&position);
        if let Ok(body) = &result { scope.visited[position] = Some(body.clone()); }
    });
    result
}
pub fn write_maybe<S: Serializer>(routine: &Option<Rc<crate::form::Routine>>, output: S) -> Result<S::Ok, S::Error> {
    let encoded = match routine { Some(body) => record_body(body).map_err(serde::ser::Error::custom)?, None => Node::Null };
    encoded.serialize(output)
}
pub fn read_maybe<'de, D: Deserializer<'de>>(input: D) -> Result<Option<Rc<crate::form::Routine>>, D::Error> {
    match Node::deserialize(input)? {
        Node::Null => Ok(None),
        encoded => body_from_record(encoded).map(Some).map_err(serde::de::Error::custom),
    }
}
pub fn write_members<S: Serializer>(row: &[(String, Rc<crate::form::Routine>)], output: S) -> Result<S::Ok, S::Error> {
    let mut records = Vec::with_capacity(row.len());
    for (name, routine) in row {
        records.push((name, record_body(routine).map_err(serde::ser::Error::custom)?));
    }
    records.serialize(output)
}
pub fn read_members<'de, D: Deserializer<'de>>(input: D) -> Result<Vec<(String, Rc<crate::form::Routine>)>, D::Error> {
    let encoded = Vec::<(String, Node)>::deserialize(input)?;
    let mut members = Vec::with_capacity(encoded.len());
    for (name, body) in encoded {
        members.push((name, body_from_record(body).map_err(serde::de::Error::custom)?));
    }
    Ok(members)
}

// Serialize this kernel's form envelope before handing bytes to the codec.
pub fn pack_forms(envelope: &serde_json::Value) -> Result<Vec<u8>, String> {
    let serialized = serde_json::to_vec(envelope).map_err(|fault| format!("ValueError: {fault}"))?;
    pack_saved_forms(&serialized)
}
pub fn pack_saved_forms(serialized: &[u8]) -> Result<Vec<u8>, String> {
    let mut packet = Vec::from(b"LCC6".as_slice());
    packet.extend(lumen_zlib::compress(serialized).map_err(|fault| format!("ValueError: {fault}"))?);
    Ok(packet)
}
pub fn unpack_forms(packet: &[u8]) -> Result<serde_json::Value, String> {
    let compressed = packet.strip_prefix(b"LCC6").ok_or("ValueError: invalid compiled forms header")?;
    let serialized = lumen_zlib::decompress(compressed).map_err(|fault| format!("ValueError: {fault}"))?;
    serde_json::from_slice(&serialized).map_err(|fault| format!("ValueError: {fault}"))
}
