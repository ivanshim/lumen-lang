// Microcode kernel, second design.
//
// Four stages, as before: ingest (text to tokens), structure (blocks),
// reduce (tokens to a tree of nine primitive forms, names resolved to
// slots) and execute (the tree to values). What is new is that the tree
// is the kernel's product, not a step on the way: it keeps its source
// lines, a postfix program is read into it with a symbolic stack, and the
// emitter (`emit.rs`) writes it back out in any language a definition
// describes. This crate never imports the other kernels.

pub mod emit;
pub mod execute;
pub mod ingest;
pub mod numeric;
pub mod reduce;
pub mod spec;
pub mod structure;
pub mod tree;
pub mod value;

use std::collections::HashMap;
use std::rc::Rc;

use spec::{Spec, Style};
use tree::Program;
use value::Value;

const EMBEDDED: &[&str] = &[
    include_str!("../../../langs/lumen.json"),
    include_str!("../../../langs/rplumen.json"),
    include_str!("../../../langs/python.json"),
    include_str!("../../../langs/rust.json"),
];

pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    EMBEDDED.iter().map(|t| spec::describe(t)).collect()
}

pub fn language_of(definition: &str) -> Result<String, String> {
    spec::describe(definition).map(|(n, _)| n)
}

/// The embedded definition text for a language name.
pub fn embedded(language: &str) -> Result<&'static str, String> {
    for text in EMBEDDED {
        if spec::describe(text)?.0 == language {
            return Ok(text);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    run_definition(embedded(language)?, source, program_args)
}

pub fn run_definition(definition: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let spec = Spec::read(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    let prefix = spec.error_prefix();
    let (program, globals) = build(&spec, source).map_err(|e| format!("{}: {}", prefix, e))?;
    execute_program(&spec, &program, globals, program_args).map_err(|e| format!("{}: {}", prefix, e))
}

/// Ingest, structure and reduce: the tree and the global names.
pub fn build(spec: &Spec, source: &str) -> Result<(Rc<Program>, reduce::Globals), String> {
    let tokens = ingest::tokens(source, spec)?;
    let tokens = structure::blocks(tokens, spec)?;
    let mut globals = reduce::Globals::default();
    for label in ["system.args", "system.memoization", "system.real_default_precision", "system.entry",
                  "system.kind.integer", "system.kind.rational", "system.kind.real", "system.kind.string",
                  "system.kind.boolean", "system.kind.array", "system.kind.null"] {
        if let Some(name) = spec.first(label) {
            globals.slot(name);
        }
    }
    if spec.style != Style::Postfix {
        let (program, _) = reduce::reduce(&tokens, spec, &mut globals, HashMap::new(), true)?;
        return Ok((program, globals));
    }
    // A postfix program is read with the arities its named programs are
    // assumed to have, and each reading finds what they really are: read
    // leniently until the assumptions hold, then once more strictly.
    let mut assumed: HashMap<String, reduce::Arity> = HashMap::new();
    for _ in 0..8 {
        let mut fresh = reduce::Globals::default();
        for name in &globals.names {
            fresh.slot(name);
        }
        let (_, found) = reduce::reduce(&tokens, spec, &mut fresh, assumed.clone(), false)?;
        let settled = found.iter().all(|(name, arity)| assumed.get(name) == Some(arity));
        if std::env::var_os("LUMEN_DEBUG_ARITY").is_some() {
            eprintln!("arities: {:?}", found);
        }
        assumed = found;
        if settled {
            let (program, _) = reduce::reduce(&tokens, spec, &mut fresh, assumed, true)?;
            return Ok((program, fresh));
        }
    }
    Err("The programs of this file take and leave values in a way that does not settle".to_string())
}

fn execute_program(spec: &Spec, program: &Rc<Program>, globals: reduce::Globals, program_args: &[String]) -> Result<(), String> {
    let mut machine = execute::Machine::new(spec, globals.names.clone());
    if let Some(name) = spec.first("system.args") {
        machine.bind_global(name, Value::from_text(&program_args.join(" ")));
    }
    for (label, tag) in [
        ("system.kind.integer", value::Tag::Integer), ("system.kind.rational", value::Tag::Rational),
        ("system.kind.real", value::Tag::Real), ("system.kind.string", value::Tag::Text),
        ("system.kind.boolean", value::Tag::Boolean), ("system.kind.array", value::Tag::Array),
        ("system.kind.null", value::Tag::Null),
    ] {
        if let Some(name) = spec.first(label) {
            machine.bind_global(name, Value::Tag(tag));
        }
    }
    if let Some(name) = spec.first("system.real_default_precision") {
        machine.bind_global(name, Value::Small(numeric::DEFAULT_DIGITS as i64));
    }
    machine.run_top(&program.body)?;
    if let Some(entry) = spec.first("system.entry") {
        if let Some(Value::Routine(main)) = machine.global(entry).cloned() {
            machine.run(&main, Vec::new())?;
        }
    }
    Ok(())
}

/// Write `source`, a program in the language of `definition`, in the
/// language of `target`.
pub fn emit(definition: &str, source: &str, target: &str) -> Result<String, String> {
    let from = Spec::read(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    let to = Spec::read(target).map_err(|e| format!("Error: target definition: {e}"))?;
    let (program, globals) = build(&from, source).map_err(|e| format!("{}: {}", from.error_prefix(), e))?;
    emit::emit(&program, &globals, &from, &to).map_err(|e| format!("Cannot write this program in {}: {}", to.name, e))
}
