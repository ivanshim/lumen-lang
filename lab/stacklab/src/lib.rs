// Stack kernel, second design: five words are the whole machine.
//
// A definition from langs/ says how a language is spelled; the assembler
// turns a program in that language into five kinds of word over a data
// stack, and the machine runs them. Everything the first stack design had
// a word for is a shape made of these five. This crate never imports the
// other kernels; they meet only in the host.

pub mod assemble;
pub mod language;
pub mod machine;
pub mod numbers;
pub mod scan;
pub mod shape;
pub mod values;
pub mod words;

use language::Def;
use values::Value;

/// Definitions embedded at build time; one given on the command line is
/// read the same way.
const EMBEDDED: &[&str] = &[
    include_str!("../../../langs/lumen.json"),
    include_str!("../../../langs/rplumen.json"),
    include_str!("../../../langs/python.json"),
    include_str!("../../../langs/rust.json"),
];

/// The embedded languages: each name with its file extensions.
pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    EMBEDDED.iter().map(|text| language::describe(text)).collect()
}

/// The name of the language a definition describes.
pub fn language_of(definition: &str) -> Result<String, String> {
    language::describe(definition).map(|(name, _)| name)
}

/// Run `source` as the embedded language `language`.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    for text in EMBEDDED {
        if language::describe(text)?.0 == language {
            let def = Def::read(text).map_err(|e| format!("Error: definition of '{language}': {e}"))?;
            return go(&def, source, program_args);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

/// Run `source` under a definition given as JSON text.
pub fn run_definition(definition: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let def = Def::read(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    go(&def, source, program_args)
}

fn go(def: &Def, source: &str, program_args: &[String]) -> Result<(), String> {
    go_inner(def, source, program_args).map_err(|e| format!("{}: {}", def.prefix, e))
}

fn go_inner(def: &Def, source: &str, program_args: &[String]) -> Result<(), String> {
    let toks = shape::shape(scan::scan(source, def)?, def)?;
    let mut table = assemble::Table::default();
    // The system names are globals whether or not the program mentions them.
    let system = [&def.args_name, &def.memo_name, &def.precision_name, &def.entry_name];
    for name in system.into_iter().flatten() {
        table.slot(name);
    }
    for (name, _) in &def.kind_names {
        table.slot(name);
    }
    let program = assemble::assemble(&toks, def, &mut table)?;

    let mut machine = machine::Machine::new(def, table.names.clone());
    if let Some(name) = &def.args_name {
        machine.set_global(name, Value::str(&program_args.join(" ")));
    }
    for (name, kind) in &def.kind_names {
        machine.set_global(name, machine::kind_value(*kind));
    }
    if let Some(name) = &def.precision_name {
        machine.set_global(name, machine::default_precision());
    }
    machine.call(&program, Vec::new())?;

    // A language with an entry function (Rust's `main`) runs it once the
    // program body has defined it.
    if let Some(entry) = &def.entry_name {
        if let Some(Value::Program(main)) = machine.global(entry).cloned() {
            machine.call(&main, Vec::new())?;
        }
    }
    Ok(())
}
