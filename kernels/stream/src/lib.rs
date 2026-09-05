// Stream kernel: a language-agnostic tree-walking interpreter substrate.
//
// `kernel/` holds the neutral machinery (lexer, token registry, parser
// navigation, AST traits, evaluator, runtime environment). `languages/` holds
// everything that gives source text meaning. This crate never imports the
// microcode kernel; the two kernels only meet in the command-line host.

pub mod kernel;
pub mod languages;

/// Run `source` as `language` on the stream kernel.
/// `program_args` are exposed to the program as its arguments.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    match language {
        "lumen" => run_lumen(source, program_args),
        "rust_core" => run_rust_core(source),
        "python_core" => run_python_core(source),
        other => Err(format!("Error: Unknown language '{}'", other)),
    }
}

fn run_lumen(source: &str, program_args: &[String]) -> Result<(), String> {
    use kernel::eval;
    use kernel::lexer::lex;
    use kernel::parser::Parser;
    use languages::lumen::registry::Registry;
    use languages::lumen::structure::structural;

    let mut registry = Registry::new();
    languages::lumen::dispatcher::register_all(&mut registry);

    let source = structural::strip_comments(source);
    let raw_tokens = lex(&source, &registry.tokens).map_err(|e| format!("LexError: {e}"))?;
    let processed_tokens = structural::process_indentation(&source, raw_tokens)
        .map_err(|e| format!("IndentationError: {e}"))?;
    let mut parser = Parser::new_with_tokens(processed_tokens, &registry.tokens)?;
    let program = structural::parse_program(&mut parser, &registry)?;

    let args_str = program_args.join(" ");
    let init_env = move |env: &mut kernel::runtime::Env| {
        use languages::lumen::values::{KindValue, LumenKind, LumenNumber, LumenString};
        use num_bigint::BigInt;

        // ARGS: the program arguments as one string; read-only by convention of the language.
        env.define("ARGS".to_string(), Box::new(LumenString::new(args_str)));

        // Kind meta-values matching what kind() returns.
        env.define("INTEGER".to_string(), Box::new(LumenKind::new(KindValue::INTEGER)));
        env.define("RATIONAL".to_string(), Box::new(LumenKind::new(KindValue::RATIONAL)));
        env.define("REAL".to_string(), Box::new(LumenKind::new(KindValue::REAL)));
        env.define("STRING".to_string(), Box::new(LumenKind::new(KindValue::STRING)));
        env.define("BOOLEAN".to_string(), Box::new(LumenKind::new(KindValue::BOOLEAN)));
        env.define("ARRAY".to_string(), Box::new(LumenKind::new(KindValue::ARRAY)));
        env.define("NULL".to_string(), Box::new(LumenKind::new(KindValue::NULL)));

        env.define("REAL_DEFAULT_PRECISION".to_string(), Box::new(LumenNumber::new(BigInt::from(15))));
        Ok(())
    };

    eval::eval(&program, init_env).map_err(|e| format!("RuntimeError: {e}"))
}

fn run_rust_core(source: &str) -> Result<(), String> {
    use kernel::eval;
    use kernel::lexer::lex;
    use kernel::parser::Parser;
    use languages::rust_core::registry::Registry;
    use languages::rust_core::structure::structural;

    let mut registry = Registry::new();
    languages::rust_core::register_all(&mut registry);

    let source = structural::strip_comments(source);
    let raw_tokens = lex(&source, &registry.tokens).map_err(|e| format!("LexError: {e}"))?;
    let processed_tokens = structural::process_tokens(raw_tokens).map_err(|e| format!("TokenError: {e}"))?;
    let mut parser = Parser::new_with_tokens(processed_tokens, &registry.tokens)?;
    let program = structural::parse_program(&mut parser, &registry)?;

    eval::eval(&program, |_env| Ok(())).map_err(|e| format!("RuntimeError: {e}"))
}

fn run_python_core(source: &str) -> Result<(), String> {
    use kernel::eval;
    use kernel::lexer::lex;
    use kernel::parser::Parser;
    use languages::python_core::registry::Registry;
    use languages::python_core::structure::structural;

    let mut registry = Registry::new();
    languages::python_core::register_all(&mut registry);

    let source = structural::strip_comments(source);
    let raw_tokens = lex(&source, &registry.tokens).map_err(|e| format!("LexError: {e}"))?;
    let processed_tokens = structural::process_indentation(&source, raw_tokens)
        .map_err(|e| format!("IndentationError: {e}"))?;
    let mut parser = Parser::new_with_tokens(processed_tokens, &registry.tokens)?;
    let program = structural::parse_program(&mut parser, &registry)?;

    eval::eval(&program, |_env| Ok(())).map_err(|e| format!("RuntimeError: {e}"))
}
