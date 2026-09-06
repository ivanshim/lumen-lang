// Stream kernel: a language-agnostic tree-walking interpreter substrate.
//
// `kernel/` holds the neutral machinery (lexer, token registry, parser
// navigation, AST traits, evaluator, runtime environment). `language/`
// holds everything that gives source text meaning: handlers written as
// code, spelled and shaped by a definition from langs/. This crate never
// imports the microcode kernel; the two kernels only meet in the host.

pub mod kernel;
pub mod language;

use language::definition::{self, Definition};

/// The embedded languages: each name with its file extensions.
pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    definition::EMBEDDED.iter().map(|text| definition::describe(text)).collect()
}

/// Run `source` as the embedded language `language`.
/// `program_args` are exposed to the program as its arguments.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    run_definition(definition::embedded(language)?, source, program_args)
}

/// Run `source` under a definition given as JSON text.
pub fn run_definition(text: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let parsed = Definition::parse(text).map_err(|e| format!("Error: language definition: {e}"))?;
    definition::install(parsed).map_err(|e| format!("Error: {e}"))?;
    run_installed(source, program_args)
}

fn run_installed(source: &str, program_args: &[String]) -> Result<(), String> {
    use kernel::eval;
    use kernel::lexer::lex;
    use kernel::parser::Parser;
    use language::definition::def;
    use language::expressions::variable::call_user_function;
    use language::registry::Registry;
    use language::statements::functions;
    use language::structure::structural;

    let d = def();
    let mut registry = Registry::new();
    language::dispatcher::register_all(&mut registry);

    let raw_tokens = lex(source, &registry.tokens).map_err(|e| format!("LexError: {e}"))?;
    let processed_tokens = structural::process(source, raw_tokens).map_err(|e| format!("StructureError: {e}"))?;
    let mut parser = Parser::new_with_tokens(processed_tokens, &registry.tokens)?;
    let program = structural::parse_program(&mut parser, &registry)?;

    let args_str = program_args.join(" ");
    let init_env = move |env: &mut kernel::runtime::Env| {
        use language::values::{KindValue, LumenKind, LumenNumber, LumenString};
        use num_bigint::BigInt;

        // The program arguments as one string; read-only by convention of the language.
        if let Some(name) = d.list("system.args").first() {
            env.define(name.clone(), Box::new(LumenString::new(args_str)));
        }

        // Kind meta-values matching what the kind builtin returns.
        let kinds = [
            ("system.kind.integer", KindValue::INTEGER),
            ("system.kind.rational", KindValue::RATIONAL),
            ("system.kind.real", KindValue::REAL),
            ("system.kind.string", KindValue::STRING),
            ("system.kind.boolean", KindValue::BOOLEAN),
            ("system.kind.array", KindValue::ARRAY),
            ("system.kind.null", KindValue::NULL),
        ];
        for (label, kind) in kinds {
            if let Some(name) = d.list(label).first() {
                env.define(name.clone(), Box::new(LumenKind::new(kind)));
            }
        }

        if let Some(name) = d.list("system.real_default_precision").first() {
            env.define(name.clone(), Box::new(LumenNumber::new(BigInt::from(15))));
        }
        Ok(())
    };

    let mut env = eval::eval(&program, init_env).map_err(|e| format!("RuntimeError: {e}"))?;

    // A language with an entry function (Rust's `main`) runs it once the
    // program body has defined it.
    if let Some(entry) = d.list("system.entry").first() {
        if functions::get_function(entry).is_some() {
            call_user_function(entry, Vec::new(), &mut env).map_err(|e| format!("RuntimeError: {e}"))?;
        }
    }
    Ok(())
}
