// Build-time packaging artifact: embedded .lm file contents
// This is not library code; it simply packages source files for embedding in the binary.

pub static EMBEDDED_FILES: &[(&str, &str)] = &[
    ("langs/lib_lumen/render.lm", include_str!("render.lm")),
    ("langs/lib_lumen/to_string.lm", include_str!("to_string.lm")),
    ("langs/lib_lumen/string_to_value.lm", include_str!("string_to_value.lm")),
    ("langs/lib_lumen/numeric.lm", include_str!("numeric.lm")),
    ("langs/lib_lumen/output.lm", include_str!("output.lm")),
    ("langs/lib_lumen/array.lm", include_str!("array.lm")),
    ("langs/lib_lumen/string.lm", include_str!("string.lm")),
    ("langs/lib_lumen/string_ord_chr.lm", include_str!("string_ord_chr.lm")),
    ("langs/lib_lumen/factorial.lm", include_str!("factorial.lm")),
    ("langs/lib_lumen/round.lm", include_str!("round.lm")),
    ("langs/lib_lumen/e_integer.lm", include_str!("e_integer.lm")),
    ("langs/lib_lumen/pi_machin.lm", include_str!("pi_machin.lm")),
    ("langs/lib_lumen/modular_arithmetic.lm", include_str!("modular_arithmetic.lm")),
    ("langs/lib_lumen/primes.lm", include_str!("primes.lm")),
    ("langs/lib_lumen/number_theory.lm", include_str!("number_theory.lm")),
    ("langs/lib_lumen/constants_1024.lm", include_str!("constants_1024.lm")),
    ("langs/lib_lumen/constants.lm", include_str!("constants.lm")),
    ("langs/lib_lumen/constants_default.lm", include_str!("constants_default.lm")),
];
