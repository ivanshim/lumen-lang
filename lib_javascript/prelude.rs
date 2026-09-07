// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as javascript spells it, embedded in the host and prepended to
// every javascript program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("lib_javascript/to_string.js", include_str!("to_string.js")),
    ("lib_javascript/numeric.js", include_str!("numeric.js")),
    ("lib_javascript/array.js", include_str!("array.js")),
    ("lib_javascript/string.js", include_str!("string.js")),
    ("lib_javascript/string_ord_chr.js", include_str!("string_ord_chr.js")),
    ("lib_javascript/factorial.js", include_str!("factorial.js")),
    ("lib_javascript/modular_arithmetic.js", include_str!("modular_arithmetic.js")),
    ("lib_javascript/primes.js", include_str!("primes.js")),
    ("lib_javascript/number_theory.js", include_str!("number_theory.js")),
];
