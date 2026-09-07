// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as c spells it, embedded in the host and prepended to
// every c program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("lib_c/array.c", include_str!("array.c")),
    ("lib_c/factorial.c", include_str!("factorial.c")),
    ("lib_c/modular_arithmetic.c", include_str!("modular_arithmetic.c")),
    ("lib_c/primes.c", include_str!("primes.c")),
    ("lib_c/number_theory.c", include_str!("number_theory.c")),
];
