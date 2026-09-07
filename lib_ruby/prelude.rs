// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as ruby spells it, embedded in the host and prepended to
// every ruby program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("lib_ruby/to_string.rb", include_str!("to_string.rb")),
    ("lib_ruby/string_to_value.rb", include_str!("string_to_value.rb")),
    ("lib_ruby/numeric.rb", include_str!("numeric.rb")),
    ("lib_ruby/array.rb", include_str!("array.rb")),
    ("lib_ruby/string.rb", include_str!("string.rb")),
    ("lib_ruby/string_ord_chr.rb", include_str!("string_ord_chr.rb")),
    ("lib_ruby/factorial.rb", include_str!("factorial.rb")),
    ("lib_ruby/modular_arithmetic.rb", include_str!("modular_arithmetic.rb")),
    ("lib_ruby/primes.rb", include_str!("primes.rb")),
    ("lib_ruby/number_theory.rb", include_str!("number_theory.rb")),
];
