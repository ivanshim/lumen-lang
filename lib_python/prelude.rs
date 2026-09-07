// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as python spells it, embedded in the host and prepended to
// every python program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "import sys";

pub static FILES: &[(&str, &str)] = &[
    ("lib_python/to_string.py", include_str!("to_string.py")),
    ("lib_python/string_to_value.py", include_str!("string_to_value.py")),
    ("lib_python/numeric.py", include_str!("numeric.py")),
    ("lib_python/array.py", include_str!("array.py")),
    ("lib_python/string.py", include_str!("string.py")),
    ("lib_python/string_ord_chr.py", include_str!("string_ord_chr.py")),
    ("lib_python/factorial.py", include_str!("factorial.py")),
    ("lib_python/round.py", include_str!("round.py")),
    ("lib_python/modular_arithmetic.py", include_str!("modular_arithmetic.py")),
    ("lib_python/primes.py", include_str!("primes.py")),
    ("lib_python/number_theory.py", include_str!("number_theory.py")),
];
