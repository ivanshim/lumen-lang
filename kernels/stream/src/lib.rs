// Stream kernel: a language-agnostic tree-walking interpreter substrate.
//
// `kernel/` holds the neutral machinery (lexer, token registry, parser
// navigation, AST traits, evaluator, runtime environment). `languages/`
// holds everything that gives source text meaning; today that is Lumen,
// whose spelling comes from configs/lumen.json. This crate never imports
// the microcode kernel; the two kernels only meet in the command-line host.

pub mod kernel;
pub mod languages;

/// Run `source` as `language` on the stream kernel.
/// `program_args` are exposed to the program as its arguments.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    match language {
        "lumen" => run_lumen(source, program_args),
        other => Err(format!("Error: the stream kernel hosts Lumen only, not '{}'", other)),
    }
}

fn run_lumen(source: &str, program_args: &[String]) -> Result<(), String> {
    use kernel::eval;
    use kernel::lexer::lex;
    use kernel::parser::Parser;
    use languages::lumen::definition::def;
    use languages::lumen::registry::Registry;
    use languages::lumen::structure::structural;

    let mut registry = Registry::new();
    languages::lumen::dispatcher::register_all(&mut registry);

    let raw_tokens = lex(source, &registry.tokens).map_err(|e| format!("LexError: {e}"))?;
    let processed_tokens = structural::process_indentation(source, raw_tokens)
        .map_err(|e| format!("IndentationError: {e}"))?;
    let mut parser = Parser::new_with_tokens(processed_tokens, &registry.tokens)?;
    let program = structural::parse_program(&mut parser, &registry)?;

    let args_str = program_args.join(" ");
    let init_env = move |env: &mut kernel::runtime::Env| {
        use languages::lumen::values::{KindValue, LumenKind, LumenNumber, LumenString};
        use num_bigint::BigInt;

        let d = def();
        // The program arguments as one string; read-only by convention of the language.
        env.define(d.first("system.args").to_string(), Box::new(LumenString::new(args_str)));

        // Kind meta-values matching what kind() returns.
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

    eval::eval(&program, init_env).map_err(|e| format!("RuntimeError: {e}"))
}
