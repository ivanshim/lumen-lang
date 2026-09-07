// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as php spells it, embedded in the host and prepended to
// every php program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "<?php";

pub static FILES: &[(&str, &str)] = &[
    ("langs/lib_php/to_string.php", include_str!("to_string.php")),
    ("langs/lib_php/string_to_value.php", include_str!("string_to_value.php")),
    ("langs/lib_php/numeric.php", include_str!("numeric.php")),
    ("langs/lib_php/array.php", include_str!("array.php")),
    ("langs/lib_php/string.php", include_str!("string.php")),
    ("langs/lib_php/string_ord_chr.php", include_str!("string_ord_chr.php")),
    ("langs/lib_php/factorial.php", include_str!("factorial.php")),
    ("langs/lib_php/modular_arithmetic.php", include_str!("modular_arithmetic.php")),
    ("langs/lib_php/primes.php", include_str!("primes.php")),
    ("langs/lib_php/number_theory.php", include_str!("number_theory.php")),
];
