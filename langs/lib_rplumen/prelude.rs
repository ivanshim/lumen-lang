// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as rplumen spells it, embedded in the host and prepended to
// every rplumen program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";
pub static EPILOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("langs/lib_rplumen/to_string.rpl", include_str!("to_string.rpl")),
    ("langs/lib_rplumen/string_to_value.rpl", include_str!("string_to_value.rpl")),
    ("langs/lib_rplumen/numeric.rpl", include_str!("numeric.rpl")),
    ("langs/lib_rplumen/array.rpl", include_str!("array.rpl")),
    ("langs/lib_rplumen/string.rpl", include_str!("string.rpl")),
    ("langs/lib_rplumen/string_ord_chr.rpl", include_str!("string_ord_chr.rpl")),
    ("langs/lib_rplumen/factorial.rpl", include_str!("factorial.rpl")),
    ("langs/lib_rplumen/round.rpl", include_str!("round.rpl")),
    ("langs/lib_rplumen/e_integer.rpl", include_str!("e_integer.rpl")),
    ("langs/lib_rplumen/pi_machin.rpl", include_str!("pi_machin.rpl")),
    ("langs/lib_rplumen/modular_arithmetic.rpl", include_str!("modular_arithmetic.rpl")),
    ("langs/lib_rplumen/primes.rpl", include_str!("primes.rpl")),
    ("langs/lib_rplumen/number_theory.rpl", include_str!("number_theory.rpl")),
    ("langs/lib_rplumen/constants_1024.rpl", include_str!("constants_1024.rpl")),
    ("langs/lib_rplumen/constants.rpl", include_str!("constants.rpl")),
];
