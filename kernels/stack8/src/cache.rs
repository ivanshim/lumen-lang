// The cache stores the kernel's compiled instruction graph, never source in place of code.
use std::cell::RefCell;
use crate::value::Rc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value as Json};
use crate::value::Value;

fn constant(value: &Value) -> Result<Json, String> {
    let (kind, data) = match value {
        Value::Null => ("none", Json::Null),
        Value::Ellipsis => ("ellipsis", Json::Null),
        Value::Blank => ("blank", Json::Null),
        Value::Gap => ("gap", Json::Null),
        Value::Fence => ("fence", Json::Null),
        Value::ByteKind(mutable, spelling) => ("byte-kind", json!([mutable, spelling])),
        Value::Stream(stderr) => ("stream", json!(stderr)),
        Value::Flag(b) => ("flag", json!(b)),
        Value::Small(n) => ("small", json!(n)),
        Value::Huge(n) => ("large", json!(n.to_string())),
        Value::Text(s) => ("text", json!(s)),
        Value::Codepoints(points) => ("unicode", json!(points)),
        Value::Bytes(bytes, mutable, spelling) if !mutable => ("bytes", json!([bytes.borrow().as_slice(), spelling])),
        Value::Tuple(values) => ("tuple", serde_json::to_value(values.as_slice()).map_err(|e| e.to_string())?),
        Value::Array(values) => ("array", serde_json::to_value(values.as_slice()).map_err(|e| e.to_string())?),
        Value::Routine(body) => ("routine", routine_reference(body)?),
        Value::Native(op, name) => ("builtin", json!([op, name])),
        Value::Real(real) => ("real", serde_json::to_value(real).map_err(|e| e.to_string())?),
        Value::Frac(frac) => ("fraction", serde_json::to_value(frac).map_err(|e| e.to_string())?),
        Value::Imaginary(n, words) => ("imaginary", json!([n.to_bits(), words])),
        Value::Complex(z) => ("complex", json!([z.real.to_bits(), z.imag.to_bits(), z.integer_fault])),
        Value::Adapter(parts) => ("adapter", serde_json::to_value((parts.0, &parts.1)).map_err(|e| e.to_string())?),
        Value::SortOf(kind) => ("kind", serde_json::to_value(kind).map_err(|e| e.to_string())?),
        Value::Words(words, marked) => ("words", json!([words, marked])),
        Value::Declined(words) => ("declined", json!(words)),
        Value::Set(members) if members.borrow().fixed => {
            let members = members.borrow();
            let items = serde_json::to_value(members.items()).map_err(|e| e.to_string())?;
            ("frozenset", json!([members.word, items]))
        }
        _ => return Err(format!("ValueError: code cache cannot store {}", value.core_kind())),
    };
    Ok(json!({"kind": kind, "data": data}))
}
impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        constant(self).map_err(serde::ser::Error::custom)?.serialize(serializer)
    }
}
fn restore(record: Json) -> Result<Value, String> {
    let kind = record["kind"].as_str().ok_or("missing constant kind")?;
    let data = record["data"].clone();
    macro_rules! read { ($ty:ty) => { serde_json::from_value::<$ty>(data).map_err(|e| e.to_string())? }; }
    Ok(match kind {
        "none" => Value::Null,
        "ellipsis" => Value::Ellipsis,
        "blank" => Value::Blank,
        "gap" => Value::Gap,
        "fence" => Value::Fence,
        "byte-kind" => { let (mutable, spelling) = read!((bool, Rc<str>)); Value::ByteKind(mutable, spelling) },
        "stream" => Value::Stream(read!(bool)),
        "flag" => Value::Flag(read!(bool)),
        "small" => Value::Small(read!(i64)),
        "large" => Value::Huge(Rc::new(read!(String).parse().map_err(|_| "bad integer")?)),
        "text" => Value::Text(read!(Rc<str>)),
        "unicode" => Value::Codepoints(Rc::new(read!(Vec<u32>))),
        "bytes" => { let (bytes, spelling) = read!((Vec<u8>, Rc<str>)); Value::Bytes(Rc::new(RefCell::new(bytes)), false, spelling) },
        "tuple" => Value::tuple(read!(Vec<Value>)),
        "array" => Value::array(read!(Vec<Value>)),
        "routine" => Value::Routine(recover_routine(data)?),
        "builtin" => { let (op, name) = read!((crate::code::Builtin, Rc<str>)); Value::Native(op, name) },
        "real" => Value::Real(Rc::new(read!(crate::value::Real))),
        "fraction" => Value::Frac(Rc::new(read!(crate::value::Frac))),
        "imaginary" => { let (n, words) = read!((u64, Rc<str>)); Value::Imaginary(f64::from_bits(n), words) },
        "complex" => { let (a, b, words) = read!((u64, u64, Rc<str>)); Value::Complex(Rc::new(crate::complex::Complex { real: f64::from_bits(a), imag: f64::from_bits(b), integer_fault: words })) },
        "adapter" => { let parts = read!((u8, Vec<Value>)); Value::Adapter(Rc::new(parts)) },
        "kind" => Value::SortOf(read!(crate::value::Sort)),
        "words" => { let (words, marked) = read!((Rc<Vec<String>>, bool)); Value::Words(words, marked) },
        "declined" => Value::Declined(read!(Rc<str>)),
        "frozenset" => {
            let (word, items) = read!((String, Vec<Value>));
            let mut members = crate::value::Members::empty(word, true);
            for item in items {
                let key = item.member_key().map_err(|kind| format!("unaddressable set member {kind}"))?;
                members.insert(key, item);
            }
            Value::Set(Rc::new(RefCell::new(members)))
        }
        _ => return Err("unknown constant kind".into()),
    })
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        restore(Json::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Default)]
struct Context {
    globe: Option<Value>,
    born: Option<Value>,
    home: Option<Option<Rc<str>>>,
    origins: Vec<Rc<[crate::lex::Token]>>,
    sources: Vec<Rc<str>>,
    routines: Vec<Json>,
    restored: Vec<Option<Rc<crate::code::Routine>>>,
    active: std::collections::HashSet<usize>,
}
thread_local! { static CONTEXT: RefCell<Vec<Context>> = const { RefCell::new(Vec::new()) }; }
pub fn globals() -> Option<Value> { CONTEXT.with(|c| c.borrow().last().and_then(|s| s.globe.clone())) }
pub fn builtins() -> Option<Value> { CONTEXT.with(|c| c.borrow().last().and_then(|s| s.born.clone())) }
pub fn home() -> Option<Option<Rc<str>>> { CONTEXT.with(|c| c.borrow().last().and_then(|s| s.home.clone())) }
struct ContextScope;
impl Drop for ContextScope { fn drop(&mut self) { CONTEXT.with(|c| { c.borrow_mut().pop(); }); } }

pub fn program(mut packet: Json, registry: &mut crate::compile::Registry) -> Result<Rc<crate::code::Routine>, String> {
    if packet["abi"] != crate::PYTHON_CACHE_ABI { return Err("ImportError: incompatible compiled code ABI".into()); }
    let mut archived: crate::compile::Registry = serde_json::from_value(packet["bindings"].take()).map_err(|e| format!("ValueError: {e}"))?;
    let places: Vec<usize> = archived.idents.iter().map(|name| registry.slot(name)).collect();
    fn relocate(node: &mut Json, places: &[usize]) -> Result<(), String> {
        match node {
            Json::Object(fields) => {
                if fields.contains_key("near") && fields.contains_key("far") && fields.contains_key("ident") {
                    let old = fields["far"].as_u64().ok_or("ValueError: invalid global address")? as usize;
                    let new = *places.get(old).ok_or("ValueError: invalid global address")?;
                    fields.insert("far".into(), json!(new));
                }
                for value in fields.values_mut() { relocate(value, places)?; }
            }
            Json::Array(items) => for item in items { relocate(item, places)?; },
            _ => {}
        }
        Ok(())
    }
    relocate(&mut packet["program"], &places)?;
    relocate(&mut packet["routines"], &places)?;
    registry.globals.extend(archived.globals.drain());
    registry.declared_outer = archived.declared_outer;
    registry.shared_args.extend(archived.shared_args);
    registry.arg_names.extend(archived.arg_names);
    registry.gives_back.extend(archived.gives_back);
    let origins = serde_json::from_value(packet["origins"].clone()).map_err(|e| format!("ValueError: {e}"))?;
    let sources = serde_json::from_value(packet.get("sources").cloned().unwrap_or(Json::Array(Vec::new()))).unwrap_or_default();
    let routines: Vec<Json> = serde_json::from_value(packet["routines"].take()).map_err(|e| format!("ValueError: {e}"))?;
    let restored = vec![None; routines.len()];
    CONTEXT.with(|c| c.borrow_mut().push(Context { globe: registry.globe.clone(), born: registry.born.clone(), home: registry.home.clone(), origins, sources, routines, restored, active: Default::default() }));
    let _scope = ContextScope;
    serde_json::from_value(packet["program"].take()).map_err(|e| format!("ValueError: {e}"))
}

type OriginTable = (Vec<Rc<[crate::lex::Token]>>, std::collections::HashMap<usize, usize>);
thread_local! { static WRITING: RefCell<Option<OriginTable>> = const { RefCell::new(None) }; }
pub fn write_origin<S: Serializer>(origin: &Rc<[crate::lex::Token]>, serializer: S) -> Result<S::Ok, S::Error> {
    let index = WRITING.with(|writer| {
        let mut writer = writer.borrow_mut();
        let (words, addresses) = writer.as_mut()?;
        let key = Rc::as_ptr(origin) as *const u8 as usize;
        let index = *addresses.entry(key).or_insert_with(|| {
            words.push(origin.clone());
            words.len() - 1
        });
        Some(index)
    });
    match index { Some(index) => index.serialize(serializer), None => origin.serialize(serializer) }
}
pub fn read_origin<'de, D: Deserializer<'de>>(reader: D) -> Result<Rc<[crate::lex::Token]>, D::Error> {
    let word = Json::deserialize(reader)?;
    if word.is_array() { return serde_json::from_value::<Vec<crate::lex::Token>>(word).map(Rc::from).map_err(serde::de::Error::custom); }
    let index = word.as_u64().ok_or_else(|| serde::de::Error::custom("invalid source origin"))?;
    CONTEXT.with(|c| c.borrow().last().and_then(|s| s.origins.get(index as usize).cloned()))
        .ok_or_else(|| serde::de::Error::custom("source origin outside cache"))
}
pub fn encoded(program: &Rc<crate::code::Routine>) -> Result<(Vec<u8>, Vec<Rc<[crate::lex::Token]>>, Vec<Vec<u8>>, Vec<Rc<str>>), String> {
    let parent = WRITING.with(|w| w.replace(Some((Vec::new(), std::collections::HashMap::new()))));
    let parent_routines = ROUTINE_OUTPUT.with(|pool| pool.replace(Some((Vec::new(), Default::default()))));
    let parent_sources = SOURCES.with(|cell| cell.replace(Some((Vec::new(), Default::default()))));
    let graph = serde_json::to_vec(program).map_err(|e| format!("ValueError: {e}"));
    let sources = SOURCES.with(|cell| cell.replace(parent_sources).unwrap().0);
    let routines = ROUTINE_OUTPUT.with(|pool| pool.replace(parent_routines).unwrap().0);
    let origins = WRITING.with(|w| w.replace(parent).unwrap().0);
    graph.map(|graph| (graph, origins, routines, sources))
}

type SourceTable = (Vec<Rc<str>>, std::collections::HashMap<usize, usize>);
thread_local! { static SOURCES: RefCell<Option<SourceTable>> = const { RefCell::new(None) }; }
pub fn write_source<S: Serializer>(text: &Rc<str>, serializer: S) -> Result<S::Ok, S::Error> {
    let index = SOURCES.with(|cell| {
        let mut cell = cell.borrow_mut();
        let (words, addresses) = cell.as_mut()?;
        let key = Rc::as_ptr(text) as *const u8 as usize;
        let index = *addresses.entry(key).or_insert_with(|| {
            words.push(text.clone());
            words.len() - 1
        });
        Some(index)
    });
    match index { Some(index) => index.serialize(serializer), None => text.serialize(serializer) }
}
pub fn read_source<'de, D: Deserializer<'de>>(reader: D) -> Result<Rc<str>, D::Error> {
    let word = Json::deserialize(reader)?;
    if let Some(text) = word.as_str() { return Ok(Rc::from(text)); }
    let index = word.as_u64().ok_or_else(|| serde::de::Error::custom("invalid source"))?;
    CONTEXT.with(|c| c.borrow().last().and_then(|s| s.sources.get(index as usize).cloned()))
        .ok_or_else(|| serde::de::Error::custom("source outside cache"))
}


pub fn move_filename(node: &mut Json, old: &str, new: &str) {
    match node {
        Json::Object(fields) => {
            if fields.get("written_in").and_then(Json::as_str) == Some(old) {
                fields.insert("written_in".into(), json!(new));
            }
            for value in fields.values_mut() { move_filename(value, old, new); }
        }
        Json::Array(items) => for item in items { move_filename(item, old, new); },
        _ => (),
    }
}


type RoutineOutput = (Vec<Vec<u8>>, std::collections::HashMap<usize, usize>);
thread_local! { static ROUTINE_OUTPUT: RefCell<Option<RoutineOutput>> = const { RefCell::new(None) }; }
fn routine_reference(body: &Rc<crate::code::Routine>) -> Result<Json, String> {
    let allocation = Rc::as_ptr(body) as usize;
    let slot = ROUTINE_OUTPUT.with(|pool| {
        let mut pool = pool.borrow_mut();
        let (rows, addresses) = pool.as_mut()?;
        if let Some(index) = addresses.get(&allocation) { return Some((*index, false)); }
        let index = rows.len();
        rows.push(Vec::new());
        addresses.insert(allocation, index);
        Some((index, true))
    });
    let Some((index, fresh)) = slot else { return serde_json::to_value(body).map_err(|e| e.to_string()); };
    if fresh {
        let encoded = serde_json::to_vec(body).map_err(|e| e.to_string())?;
        ROUTINE_OUTPUT.with(|pool| pool.borrow_mut().as_mut().unwrap().0[index] = encoded);
    }
    Ok(json!(index))
}
fn recover_routine(record: Json) -> Result<Rc<crate::code::Routine>, String> {
    let Some(index) = record.as_u64() else {
        return serde_json::from_value(record).map_err(|e| e.to_string());
    };
    let index = usize::try_from(index).map_err(|_| "invalid routine index")?;
    let (ready, body) = CONTEXT.with(|context| -> Result<_, String> {
        let mut context = context.borrow_mut();
        let current = context.last_mut().ok_or("routine reference without cache")?;
        let body = current.routines.get(index).ok_or("routine outside cache")?;
        if let Some(ready) = &current.restored[index] { return Ok((Some(ready.clone()), Json::Null)); }
        if !current.active.insert(index) { return Err("cyclic routine cache".into()); }
        Ok((None, body.clone()))
    })?;
    if let Some(ready) = ready { return Ok(ready); }
    let result = serde_json::from_value::<Rc<crate::code::Routine>>(body).map_err(|e| e.to_string());
    CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let current = context.last_mut().unwrap();
        current.active.remove(&index);
        if let Ok(ready) = &result { current.restored[index] = Some(ready.clone()); }
    });
    result
}
pub fn save_optional<S: Serializer>(value: &Option<Rc<crate::code::Routine>>, writer: S) -> Result<S::Ok, S::Error> {
    value.as_ref().map(routine_reference).transpose().map_err(serde::ser::Error::custom)?.serialize(writer)
}
pub fn load_optional<'de, D: Deserializer<'de>>(reader: D) -> Result<Option<Rc<crate::code::Routine>>, D::Error> {
    let item = Json::deserialize(reader)?;
    if item.is_null() { Ok(None) } else { recover_routine(item).map(Some).map_err(serde::de::Error::custom) }
}
pub fn save_methods<S: Serializer>(methods: &[(String, Rc<crate::code::Routine>)], writer: S) -> Result<S::Ok, S::Error> {
    methods.iter().map(|(name, body)| routine_reference(body).map(|body| (name, body)))
        .collect::<Result<Vec<_>, _>>().map_err(serde::ser::Error::custom)?.serialize(writer)
}
pub fn load_methods<'de, D: Deserializer<'de>>(reader: D) -> Result<Vec<(String, Rc<crate::code::Routine>)>, D::Error> {
    Vec::<(String, Json)>::deserialize(reader)?.into_iter()
        .map(|(name, body)| recover_routine(body).map(|body| (name, body)))
        .collect::<Result<Vec<_>, _>>().map_err(serde::de::Error::custom)
}

// Cache framing belongs to this kernel; the codec only handles opaque bytes.
pub fn pack_json(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    let raw = serde_json::to_vec(value).map_err(|error| format!("ValueError: {error}"))?;
    pack_text(&raw)
}

fn append_field(out: &mut Vec<u8>, key: &str, raw: &[u8]) {
    if out.len() > 1 { out.push(b','); }
    out.push(b'"');
    out.extend_from_slice(key.as_bytes());
    out.push(b'"');
    out.push(b':');
    out.extend_from_slice(raw);
}
fn append_value(out: &mut Vec<u8>, key: &str, value: &(impl Serialize + ?Sized)) -> Result<(), String> {
    let raw = serde_json::to_vec(value).map_err(|error| format!("ValueError: {error}"))?;
    append_field(out, key, &raw);
    Ok(())
}
/// Nested routines are stored as JSON text. A value tree of every routine is
/// several times larger than that text and exhausts a large module.
pub fn pack_packet<B: Serialize, O: Serialize>(file: &str, mode: i64, function: bool, graph: &[u8], bindings: &B, origins: &O, routines: &[Vec<u8>], sources: &[Rc<str>]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.push(b'{');
    append_value(&mut out, "abi", &crate::PYTHON_CACHE_ABI)?;
    append_value(&mut out, "file", &file)?;
    append_value(&mut out, "mode", &mode)?;
    if function { append_value(&mut out, "function", &true)?; }
    append_field(&mut out, "program", graph);
    append_value(&mut out, "bindings", bindings)?;
    append_value(&mut out, "origins", origins)?;
    if out.len() > 1 { out.push(b','); }
    out.extend_from_slice(b"\"routines\":[");
    for (index, routine) in routines.iter().enumerate() {
        if index > 0 { out.push(b','); }
        out.extend_from_slice(routine);
    }
    out.push(b']');
    append_value(&mut out, "sources", sources)?;
    out.push(b'}');
    let bytes = pack_text(&out)?;
    if bytes.len() >= 256 * 1024 {
        #[cfg(target_os = "linux")]
        unsafe {
            extern "C" { fn malloc_trim(pad: usize) -> i32; }
            malloc_trim(0);
        }
    }
    Ok(bytes)
}
pub fn pack_text(raw: &[u8]) -> Result<Vec<u8>, String> {
    let compressed = lumen_zlib::compress(raw).map_err(|error| format!("ValueError: {error}"))?;
    let mut bytes = b"LCC6".to_vec();
    bytes.extend_from_slice(&compressed);
    Ok(bytes)
}
pub fn unpack_json(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let raw = bytes.strip_prefix(b"LCC6").ok_or("ValueError: invalid compiled cache header")?;
    let decoded = lumen_zlib::decompress(raw).map_err(|error| format!("ValueError: {error}"))?;
    // serde_json refuses nesting past 128. A module this compiler just wrote can be deeper.
    let mut decoder = serde_json::Deserializer::from_slice(&decoded);
    decoder.disable_recursion_limit();
    let value = Json::deserialize(&mut decoder).map_err(|error| format!("ValueError: {error}"))?;
    decoder.end().map_err(|error| format!("ValueError: {error}"))?;
    Ok(value)
}
