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
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    for text in BUILT_IN {
        if lang::identify(text)?.0 == language {
            let lang = Lang::parse(text).map_err(|e| format!("Error: definition of '{language}': {e}"))?;
            return go(&lang, source, program_args);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

/// Run `source` under a definition given as JSON text.
pub fn run_definition(definition: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let lang = Lang::parse(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    go(&lang, source, program_args)
}

fn go(lang: &Lang, source: &str, program_args: &[String]) -> Result<(), String> {
    go_inner(lang, source, program_args).map_err(|e| format!("{}: {}", lang.banner, e))
}

fn go_inner(lang: &Lang, source: &str, program_args: &[String]) -> Result<(), String> {
    let tokens = layout::layout(lex::lex(source, lang)?, lang)?;
    let mut registry = compile::Registry::default();
    // The system names are globals whether or not the program mentions them.
    let system = [&lang.args_binding, &lang.memo_binding, &lang.precision_binding, &lang.entry_binding];
    for name in system.into_iter().flatten() {
        registry.slot(name);
    }
    for (name, _) in &lang.sort_bindings {
        registry.slot(name);
    }
    let program = compile::compile(&tokens, lang, &mut registry)?;

    let mut machine = engine::Engine::new(lang, registry.idents.clone());
    if let Some(name) = &lang.args_binding {
        machine.define(name, Value::text(&program_args.join(" ")));
    }
    for (name, kind) in &lang.sort_bindings {
        machine.define(name, engine::sort_value(*kind));
    }
    if let Some(name) = &lang.precision_binding {
        machine.define(name, engine::places_default());
    }
    // A value raised and never caught is a fault like any other, told
    // in the language's own words.
    machine.invoke(&program, Vec::new()).map_err(|f| f.told(&machine.names()))?;

    // A language with an entry function (Rust's `main`) runs it once the
    // program body has defined it.
    if let Some(entry) = &lang.entry_binding {
        if let Some(Value::Routine(main)) = machine.lookup(entry).cloned() {
            machine.invoke(&main, Vec::new()).map_err(|f| f.told(&machine.names()))?;
        }
    }
    Ok(())
}
