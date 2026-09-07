// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as pascal spells it, embedded in the host and prepended to
// every pascal program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";
pub static EPILOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("langs/lib_pascal/render.pas", include_str!("render.pas")),
    ("langs/lib_pascal/string_to_value.pas", include_str!("string_to_value.pas")),
    ("langs/lib_pascal/array.pas", include_str!("array.pas")),
    ("langs/lib_pascal/string.pas", include_str!("string.pas")),
    ("langs/lib_pascal/string_ord_chr.pas", include_str!("string_ord_chr.pas")),
    ("langs/lib_pascal/factorial.pas", include_str!("factorial.pas")),
    ("langs/lib_pascal/round.pas", include_str!("round.pas")),
    ("langs/lib_pascal/modular_arithmetic.pas", include_str!("modular_arithmetic.pas")),
    ("langs/lib_pascal/primes.pas", include_str!("primes.pas")),
    ("langs/lib_pascal/number_theory.pas", include_str!("number_theory.pas")),
];
