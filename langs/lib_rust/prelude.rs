// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as rust spells it, embedded in the host and prepended to
// every rust program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("langs/lib_rust/array.rs", include_str!("array.rs")),
    ("langs/lib_rust/string.rs", include_str!("string.rs")),
    ("langs/lib_rust/factorial.rs", include_str!("factorial.rs")),
    ("langs/lib_rust/modular_arithmetic.rs", include_str!("modular_arithmetic.rs")),
    ("langs/lib_rust/primes.rs", include_str!("primes.rs")),
    ("langs/lib_rust/number_theory.rs", include_str!("number_theory.rs")),
];
