// Microcode kernel: the four stages and the runtime they share.

pub mod instruction;
pub mod value;
pub mod numeric;
pub mod env;

pub mod _1_ingest;
pub mod _2_structure;
pub mod _3_reduce;
pub mod _4_execute;

use num_bigint::BigInt;

use crate::schema::{KindName, LanguageSchema};
use env::Environment;
use value::{KindValue, Value};

/// Run a program: ingest → structure → reduce → execute.
pub fn run(source: &str, schema: &LanguageSchema, program_args: &[String]) -> Result<Value, String> {
    let tokens = _1_ingest::lex(source, schema)?;
    let tokens = _2_structure::process(tokens, schema)?;
    let program = _3_reduce::parse(&tokens, schema)?;

    let mut env = Environment::new();
    seed_system_bindings(&mut env, schema, program_args);

    let (value, _flow) = _4_execute::execute(&program, &mut env, schema)?;
    Ok(value)
}

/// Bind the system values the schema names: program arguments, kind
/// meta-values and integer constants. The kernel supplies the values; the
/// schema supplies the names.
fn seed_system_bindings(env: &mut Environment, schema: &LanguageSchema, program_args: &[String]) {
    if let Some(name) = &schema.system.args {
        env.bind(name.clone(), Value::String(program_args.join(" ")));
    }
    for (name, kind) in &schema.system.kinds {
        let kind = match kind {
            KindName::Integer => KindValue::Integer,
            KindName::Rational => KindValue::Rational,
            KindName::Real => KindValue::Real,
            KindName::String => KindValue::String,
            KindName::Boolean => KindValue::Boolean,
            KindName::Array => KindValue::Array,
            KindName::Null => KindValue::Null,
        };
        env.bind(name.clone(), Value::Kind(kind));
    }
    for (name, n) in &schema.system.integer_constants {
        env.bind(name.clone(), Value::Number(BigInt::from(*n)));
    }
}
