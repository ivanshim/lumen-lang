// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as swift spells it, embedded in the host and prepended to
// every swift program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "";

pub static FILES: &[(&str, &str)] = &[
    ("lib_swift/numeric.swift", include_str!("numeric.swift")),
    ("lib_swift/array.swift", include_str!("array.swift")),
    ("lib_swift/string.swift", include_str!("string.swift")),
    ("lib_swift/factorial.swift", include_str!("factorial.swift")),
    ("lib_swift/modular_arithmetic.swift", include_str!("modular_arithmetic.swift")),
    ("lib_swift/primes.swift", include_str!("primes.swift")),
    ("lib_swift/number_theory.swift", include_str!("number_theory.swift")),
];
