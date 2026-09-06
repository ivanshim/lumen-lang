// Microcode kernel, third design: four primitive forms.
//
// Literal, Load, Assign, Call. Sequencing, branching, loops, blocks and
// the three exits are all calls: of `last`, of `if` with program values
// for arms, of a program that calls itself, of a program run at once, of
// `return`, `break` and `continue`. The executor gives program calls in
// tail position no native stack, which is what makes the loops loops.
// This crate never imports the other kernels.

pub mod arith;
pub mod blocks;
pub mod lexer;
pub mod reduce;
pub mod run;
pub mod spec;
pub mod tree;
pub mod value;

use std::collections::HashMap;

use spec::{Layout, Spec};
use value::{Sort, Value};

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

pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    for text in EMBEDDED {
        if spec::describe(text)?.0 == language {
            return run_definition(text, source, program_args);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

pub fn run_definition(definition: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let spec = Spec::read(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    let prefix = spec.error_prefix();
    go(&spec, source, program_args).map_err(|e| format!("{}: {}", prefix, e))
}

fn go(spec: &Spec, source: &str, program_args: &[String]) -> Result<(), String> {
    let toks = lexer::lex(source, spec)?;
    let toks = blocks::shape(toks, spec)?;
    let system = ["system.args", "system.memoization", "system.real_default_precision", "system.entry", "system.kind.integer",
        "system.kind.rational", "system.kind.real", "system.kind.string", "system.kind.boolean", "system.kind.array", "system.kind.null"];
    let seeded: Vec<String> = system.iter().filter_map(|k| spec.one(k).map(str::to_string)).collect();
    let reduced = if spec.layout != Layout::Postfix {
        reduce::reduce(&toks, spec, &seeded, HashMap::new(), true)?
    } else {
        // Read leniently until the named programs' arities settle, then strictly.
        let mut assumed: HashMap<String, reduce::Arity> = HashMap::new();
        let mut settled = None;
        for _ in 0..8 {
            let r = reduce::reduce(&toks, spec, &seeded, assumed.clone(), false)?;
            let same = r.found.iter().all(|(n, a)| assumed.get(n) == Some(a));
            assumed = r.found;
            if same {
                settled = Some(reduce::reduce(&toks, spec, &seeded, assumed.clone(), true)?);
                break;
            }
        }
        settled.ok_or_else(|| "The programs of this file take and leave values in a way that does not settle".to_string())?
    };
    let mut runner = run::Runner::new(spec, reduced.global_names.clone());
    if let Some(n) = spec.one("system.args") {
        runner.set(n, Value::str(&program_args.join(" ")));
    }
    for (key, sort) in [("system.kind.integer", Sort::Integer), ("system.kind.rational", Sort::Rational), ("system.kind.real", Sort::Real),
        ("system.kind.string", Sort::Text), ("system.kind.boolean", Sort::Boolean), ("system.kind.array", Sort::Array), ("system.kind.null", Sort::Null)] {
        if let Some(n) = spec.one(key) {
            runner.set(n, Value::Sort(sort));
        }
    }
    if let Some(n) = spec.one("system.real_default_precision") {
        runner.set(n, Value::Int(arith::DIGITS as i64));
    }
    runner.run_top(&reduced.program.body)?;
    if let Some(entry) = spec.one("system.entry") {
        if let Some(Value::Closure(p, env)) = runner.get(entry) {
            runner.call(p, env, Vec::new()).map_err(|e| match e {
                run::Signal::Fail(m) => m,
                _ => String::new(),
            })?;
        }
    }
    Ok(())
}
