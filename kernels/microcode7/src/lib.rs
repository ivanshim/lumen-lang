// Microcode kernel, fourth design: seven forms, the four of microcode4
// and three the kernel lab measured worth a form of their own.
//
// Const, Read, Write, Apply: a constant, a binding read, a binding
// written, a call of a primitive or of a routine value. Cycle: a loop
// run in the frame it appears in. Dyad: an operator on two operands read
// without a visit to a form. Bump: a binding stepped in place. Branching,
// blocks and the three exits are still calls: of `choose` with routine
// values for arms, of a routine run at once, of `yield`, `leave` and
// `resume`. This crate never imports the other kernels.

pub mod math;
pub mod indent;
pub mod scan;
pub mod build;
pub mod exec;
pub mod table;
pub mod form;
pub mod data;

use std::collections::HashMap;

use table::Table;
use data::{Kind, Value};

const EMBEDDED: &[&str] = &[
    include_str!("../../../langs/lumen.json"),
    include_str!("../../../langs/rplumen.json"),
    include_str!("../../../langs/python.json"),
    include_str!("../../../langs/rust.json"),
];

pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    EMBEDDED.iter().map(|t| table::identify(t)).collect()
}

pub fn language_of(definition: &str) -> Result<String, String> {
    table::identify(definition).map(|(n, _)| n)
}

pub fn run(language: &str, source: &str, program_args: &[String], request: &[(String, String, String)]) -> Result<(), String> {
    for text in EMBEDDED {
        if table::identify(text)?.0 == language {
            return run_definition(text, source, program_args, request);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

pub fn run_definition(definition: &str, source: &str, program_args: &[String], request: &[(String, String, String)]) -> Result<(), String> {
    let table = Table::parse(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    let prefix = table.banner();
    go(&table, source, program_args, request).map_err(|e| format!("{}: {}", prefix, e))
}

/// Which group of the request each label names, and the one that holds
/// what the query, the form and the cookies carry together.
const REQUEST_PARTS: [(&str, &str); 7] = [
    ("GET", "ext.system.request.query"), ("POST", "ext.system.request.form"), ("COOKIE", "ext.system.request.cookies"),
    ("SERVER", "ext.system.request.server"), ("ENV", "ext.system.request.env"), ("FILES", "ext.system.request.files"),
    ("ALL", "ext.system.request.all"),
];

fn go(table: &Table, source: &str, program_args: &[String], request: &[(String, String, String)]) -> Result<(), String> {
    let tokens = scan::scan(source, table)?;
    let tokens = indent::indent(tokens, table)?;
    let system = ["system.args", "system.memoization", "system.real_default_precision", "system.entry", "system.kind.integer",
        "system.kind.rational", "system.kind.real", "system.kind.string", "system.kind.boolean", "system.kind.array", "system.kind.null"];
    let mut seeded: Vec<String> = system.iter().filter_map(|k| table.single(k).map(str::to_string)).collect();
    seeded.extend(REQUEST_PARTS.iter().filter_map(|(_, key)| table.single(key).map(str::to_string)));
    let reduced = if !table.rpn {
        build::build(&tokens, table, &seeded, HashMap::new(), true)?
    } else {
        // Read leniently until the named programs' arities settle, then strictly.
        let mut assumed: HashMap<String, build::Signature> = HashMap::new();
        let mut settled = None;
        for _ in 0..8 {
            let r = build::build(&tokens, table, &seeded, assumed.clone(), false)?;
            let same = r.seen.iter().all(|(n, a)| assumed.get(n) == Some(a));
            assumed = r.seen;
            if same {
                settled = Some(build::build(&tokens, table, &seeded, assumed.clone(), true)?);
                break;
            }
        }
        settled.ok_or_else(|| "The programs of this file take and leave values in a way that does not settle".to_string())?
    };
    let mut machine = exec::Machine::new(table, reduced.globals.clone());
    if let Some(n) = table.single("system.args") {
        machine.define(n, Value::text(&program_args.join(" ")));
    }
    // What the request carries, each group a map under the name the
    // definition gives it.
    for (group, key) in REQUEST_PARTS {
        let Some(name) = table.single(key) else { continue };
        let wanted: Vec<&str> = if group == "ALL" { vec!["GET", "POST", "COOKIE"] } else { vec![group] };
        let mut carried: Vec<(Value, Value)> = Vec::new();
        for (_, field, value) in request.iter().filter(|(from, _, _)| wanted.contains(&from.as_str())) {
            let field = Value::text(field);
            match carried.iter_mut().find(|(k, _)| k.equals(&field)) {
                Some(place) => place.1 = Value::text(value),
                None => carried.push((field, Value::text(value))),
            }
        }
        machine.define(name, Value::Dict(std::rc::Rc::new(carried)));
    }
    for (key, sort) in [("system.kind.integer", Kind::Whole), ("system.kind.rational", Kind::Fraction), ("system.kind.real", Kind::Decimal),
        ("system.kind.string", Kind::Chars), ("system.kind.boolean", Kind::Truth), ("system.kind.array", Kind::Vector), ("system.kind.null", Kind::Nothing)] {
        if let Some(n) = table.single(key) {
            machine.define(n, Value::KindOf(sort));
        }
    }
    if let Some(n) = table.single("system.real_default_precision") {
        machine.define(n, Value::Small(math::DEFAULT_PLACES as i64));
    }
    machine.run_main(&reduced.program.body)?;
    if let Some(entry) = table.single("system.entry") {
        if let Some(Value::Bound(p, env)) = machine.lookup(entry) {
            machine.invoke(p, env, Vec::new()).map_err(|e| match e {
                exec::Escape::Error(m) => m,
                _ => String::new(),
            })?;
        }
    }
    Ok(())
}
