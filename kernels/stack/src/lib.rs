// Stack kernel: every language compiles to one stack machine.
//
// A definition from langs/ says how a language is spelled; the compiler
// turns a program in that language into words over a data stack, and the
// machine runs the words. RPLumen is the machine's own notation; the
// infix languages compile to it. This crate never imports the other
// kernels; the three meet only in the host.

pub mod code;
pub mod compiler;
pub mod definition;
pub mod layout;
pub mod lexer;
pub mod machine;
pub mod number;
pub mod value;

use std::rc::Rc;

use definition::Language;
use value::Value;

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
    EMBEDDED.iter().map(|text| definition::describe(text)).collect()
}

/// The name of the language a definition describes.
pub fn language_of(definition: &str) -> Result<String, String> {
    definition::describe(definition).map(|(name, _)| name)
}

/// Run `source` as the embedded language `language`.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    for text in EMBEDDED {
        if definition::describe(text)?.0 == language {
            let lang = Language::read(text).map_err(|e| format!("Error: definition of '{language}': {e}"))?;
            return run_language(&lang, source, program_args);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

/// Run `source` under a definition given as JSON text.
pub fn run_definition(definition: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let lang = Language::read(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    run_language(&lang, source, program_args)
}

fn run_language(lang: &Language, source: &str, program_args: &[String]) -> Result<(), String> {
    run_inner(lang, source, program_args).map_err(|e| format!("{}: {}", lang.error_prefix, e))
}

fn run_inner(lang: &Language, source: &str, program_args: &[String]) -> Result<(), String> {
    let tokens = lexer::scan(source, lang)?;
    let tokens = layout::shape(tokens, lang)?;
    let mut globals = compiler::Globals::default();
    // The system names are globals whether or not the program mentions them.
    for name in [&lang.args_name, &lang.memo_name, &lang.precision_name, &lang.entry_name].into_iter().flatten() {
        globals.slot(name);
    }
    for (name, _) in &lang.kind_names {
        globals.slot(name);
    }
    let program = compiler::compile(&tokens, lang, &mut globals)?;

    let mut machine = machine::Machine::new(lang, globals.names.clone());
    if let Some(name) = &lang.args_name {
        machine.set_global(name, Value::text(&program_args.join(" ")));
    }
    for (name, kind) in &lang.kind_names {
        machine.set_global(name, machine::kind_value(*kind));
    }
    if let Some(name) = &lang.precision_name {
        machine.set_global(name, machine::default_precision());
    }
    machine.run(&program, Vec::new())?;

    // A language with an entry function (Rust's `main`) runs it once the
    // program body has defined it.
    if let Some(entry) = &lang.entry_name {
        if let Some(Value::Program(main)) = machine.global(entry).cloned() {
            let main: Rc<code::Program> = main;
            machine.run(&main, Vec::new())?;
        }
    }
    Ok(())
}
