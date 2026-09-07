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

pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    for text in EMBEDDED {
        if table::identify(text)?.0 == language {
            return run_definition(text, source, program_args);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

pub fn run_definition(definition: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let table = Table::parse(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    let prefix = table.banner();
    go(&table, source, program_args).map_err(|e| format!("{}: {}", prefix, e))
}

fn go(table: &Table, source: &str, program_args: &[String]) -> Result<(), String> {
    let tokens = scan::scan(source, table)?;
    let tokens = indent::indent(tokens, table)?;
    let system = ["system.args", "system.memoization", "system.real_default_precision", "system.entry", "system.kind.integer",
        "system.kind.rational", "system.kind.real", "system.kind.string", "system.kind.boolean", "system.kind.array", "system.kind.null"];
    let seeded: Vec<String> = system.iter().filter_map(|k| table.single(k).map(str::to_string)).collect();
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
