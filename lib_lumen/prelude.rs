// Build-time packaging artifact: embedded .lm file contents
// This is not library code; it simply packages source files for embedding in the binary.

pub static EMBEDDED_FILES: &[(&str, &str)] = &[
    ("lib_lumen/str.lm", include_str!("str.lm")),
    ("lib_lumen/numeric.lm", include_str!("numeric.lm")),
    ("lib_lumen/output.lm", include_str!("output.lm")),
    ("lib_lumen/string.lm", include_str!("string.lm")),
    ("lib_lumen/string_ord_chr.lm", include_str!("string_ord_chr.lm")),
    ("lib_lumen/numeric_to_base_string.lm", include_str!("numeric_to_base_string.lm")),
    ("lib_lumen/factorial.lm", include_str!("factorial.lm")),
    ("lib_lumen/round.lm", include_str!("round.lm")),
    ("lib_lumen/e_integer.lm", include_str!("e_integer.lm")),
    ("lib_lumen/pi_machin.lm", include_str!("pi_machin.lm")),
    ("lib_lumen/primes.lm", include_str!("primes.lm")),
    ("lib_lumen/number_theory.lm", include_str!("number_theory.lm")),
    ("lib_lumen/constants_1024.lm", include_str!("constants_1024.lm")),
    ("lib_lumen/constants.lm", include_str!("constants.lm")),
    ("lib_lumen/constants_default.lm", include_str!("constants_default.lm")),
];
