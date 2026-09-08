// Stack kernel, third design: eight words, the five of stack5 and three
// fused from them where the measurements said a word earns its place.
//
// A definition from langs/ says how a language is spelled; the assembler
// turns a program in that language into five kinds of word over a data
// stack, and the machine runs them. Everything the first stack design had
// a word for is a shape made of these five. This crate never imports the
// other kernels; they meet only in the host.

pub mod compile;
pub mod lang;
pub mod engine;
pub mod arith;
pub mod lex;
pub mod layout;
pub mod value;
pub mod code;

use lang::Lang;
use value::Value;

/// Definitions embedded at build time; one given on the command line is
/// read the same way.
const BUILT_IN: &[&str] = &[
    include_str!("../../../langs/lumen.json"),
    include_str!("../../../langs/rplumen.json"),
    include_str!("../../../langs/python.json"),
    include_str!("../../../langs/rust.json"),
];

/// The embedded languages: each name with its file extensions.
pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    BUILT_IN.iter().map(|text| lang::identify(text)).collect()
}

/// The name of the language a definition describes.
pub fn language_of(definition: &str) -> Result<String, String> {
    lang::identify(definition).map(|(name, _)| name)
}

/// Run `source` as the embedded language `language`.
pub fn run(language: &str, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    for text in BUILT_IN {
        if lang::identify(text)?.0 == language {
            let lang = Lang::parse(text).map_err(|e| format!("Error: definition of '{language}': {e}"))?;
            return go(&lang, source, program_args, request);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

/// Run `source` under a definition given as JSON text.
pub fn run_definition(definition: &str, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    let lang = Lang::parse(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    go(&lang, source, program_args, request)
}

fn go(lang: &Lang, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    go_inner(lang, source, program_args, request).map_err(|e| format!("{}: {}", lang.banner, e))
}

fn go_inner(lang: &Lang, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    let tokens = layout::layout(lex::lex(source, lang)?, lang)?;
    let mut registry = compile::Registry::default();
    // The system names are globals whether or not the program mentions them.
    let system = [&lang.args_binding, &lang.memo_binding, &lang.precision_binding, &lang.entry_binding];
    for name in system.into_iter().flatten() {
        registry.slot(name);
    }
    if !lang.kind_spelled {
        for (name, _) in &lang.sort_bindings {
            registry.slot(name);
        }
    }
    for (_, name) in &lang.request_bindings {
        registry.slot(name);
    }
    for (_, name) in &lang.source_bindings {
        registry.slot(name);
    }
    let before = request
        .iter()
        .find(|(from, key, ..)| from == "SELF" && key == "lines_before")
        .and_then(|(.., n, _)| n.parse().ok())
        .unwrap_or(0);
    let program = compile::compile(&tokens, lang, &mut registry, before)?;

    let mut machine = engine::Engine::new(lang, registry);
    if let Some(name) = &lang.args_binding {
        machine.define(name, Value::text(&program_args.join(" ")));
    }
    // Every part of the request is a map of what it carries, under the
    // name the definition gives it; the one for all of them holds what
    // the query, the form and the cookies carry together.
    for (group, name) in &lang.request_bindings {
        let wanted: Vec<&str> = match group.as_str() {
            "ALL" => vec!["GET", "POST", "COOKIE"],
            other => vec![other],
        };
        let mut carried: Vec<(Value, Value)> = Vec::new();
        for (_, key, value, counted) in request.iter().filter(|(from, ..)| wanted.contains(&from.as_str())) {
            let steps: Vec<&str> = key.split('\u{1f}').collect();
            let held = match counted {
                true => Value::Small(value.parse().unwrap_or(0)),
                false => Value::text(value),
            };
            put_step(&mut carried, &steps, held);
        }
        machine.define(name, Value::Map(std::rc::Rc::new(carried)));
    }
    if let Some((.., place, _)) = request.iter().find(|(from, key, ..)| from == "SELF" && key == "file") {
        machine.written_in(place);
    }
    // Where the program is written, as the request carries it.
    for (part, name) in &lang.source_bindings {
        let found = request.iter().find(|(from, key, ..)| from == "SELF" && key == part);
        if let Some((.., value, _)) = found {
            machine.define(name, Value::text(value));
        }
    }
    if !lang.kind_spelled {
        for (name, kind) in &lang.sort_bindings {
            machine.define(name, engine::sort_value(*kind));
        }
    }
    if let Some(name) = &lang.precision_binding {
        machine.define(name, engine::places_default());
    }
    // A value raised and never caught is a fault like any other, told
    // in the language's own words.
    if let Err(fault) = machine.invoke(&program, Vec::new()) {
        // A run the program itself said was over came out right.
        if matches!(fault, engine::Fault::Finished) {
            return Ok(());
        }
        machine.ended_uncaught(&fault);
        return Err(fault.told(&machine.names()));
    }

    // A language with an entry function (Rust's `main`) runs it once the
    // program body has defined it.
    if let Some(entry) = &lang.entry_binding {
        if let Some(Value::Routine(main)) = machine.lookup(entry).cloned() {
            machine.invoke(&main, Vec::new()).map_err(|f| f.told(&machine.names()))?;
        }
    }
    Ok(())
}

/// Put a value where a name points: each step names a place in a map,
/// and a step with no name is the next whole-number place. The maps
/// along the way are made as they are needed.
fn put_step(into: &mut Vec<(Value, Value)>, steps: &[&str], value: Value) {
    let next = || {
        let highest = into
            .iter()
            .filter_map(|(k, _)| match k {
                Value::Small(n) => Some(n + 1),
                _ => None,
            })
            .max();
        Value::Small(highest.unwrap_or(0).max(0))
    };
    let key = match steps.first() {
        // A step of digits names a whole-number place, not one named
        // by that text, so `a[]` and `a[0]` name the same place.
        Some(step) if step.chars().all(|c| c.is_ascii_digit()) && !step.is_empty() => {
            Value::Small(step.parse().unwrap_or(0))
        }
        Some(step) if !step.is_empty() => Value::text(step),
        _ => next(),
    };
    if steps.len() <= 1 {
        match into.iter_mut().find(|(k, _)| k.equals(&key)) {
            Some(place) => place.1 = value,
            None => into.push((key, value)),
        }
        return;
    }
    let at = match into.iter().position(|(k, _)| k.equals(&key)) {
        Some(at) => at,
        None => {
            into.push((key, Value::Map(std::rc::Rc::new(Vec::new()))));
            into.len() - 1
        }
    };
    let mut inside = match &into[at].1 {
        Value::Map(pairs) => pairs.as_ref().clone(),
        _ => Vec::new(),
    };
    put_step(&mut inside, &steps[1..], value);
    into[at].1 = Value::Map(std::rc::Rc::new(inside));
}
