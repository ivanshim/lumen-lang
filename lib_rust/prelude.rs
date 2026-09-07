// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as rust spells it, embedded in the host and prepended to
// every rust program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("lib_rust/array.rs", include_str!("array.rs")),
    ("lib_rust/string.rs", include_str!("string.rs")),
    ("lib_rust/factorial.rs", include_str!("factorial.rs")),
    ("lib_rust/modular_arithmetic.rs", include_str!("modular_arithmetic.rs")),
    ("lib_rust/primes.rs", include_str!("primes.rs")),
    ("lib_rust/number_theory.rs", include_str!("number_theory.rs")),
];
