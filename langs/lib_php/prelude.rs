// Build-time packaging artifact, written by scripts/port_examples.py: the
// Lumen library as php spells it, embedded in the host and prepended to
// every php program. Not library code; edit lib_lumen/ instead.

pub static PROLOGUE: &str = "<?php";
pub static EPILOGUE: &str = "?>";

pub static FILES: &[(&str, &str)] = &[
    ("langs/lib_php/native/arrays.php", include_str!("native/arrays.php")),
    ("langs/lib_php/native/classes.php", include_str!("native/classes.php")),
    ("langs/lib_php/native/contexts.php", include_str!("native/contexts.php")),
    ("langs/lib_php/native/datetime.php", include_str!("native/datetime.php")),
    ("langs/lib_php/native/exceptions.php", include_str!("native/exceptions.php")),
    ("langs/lib_php/native/files.php", include_str!("native/files.php")),
    ("langs/lib_php/native/iterators.php", include_str!("native/iterators.php")),
    ("langs/lib_php/native/json.php", include_str!("native/json.php")),
    ("langs/lib_php/native/markup.php", include_str!("native/markup.php")),
    ("langs/lib_php/native/random.php", include_str!("native/random.php")),
    ("langs/lib_php/native/regex.php", include_str!("native/regex.php")),
    ("langs/lib_php/native/runtime.php", include_str!("native/runtime.php")),
    ("langs/lib_php/native/serialize.php", include_str!("native/serialize.php")),
    ("langs/lib_php/native/sessions.php", include_str!("native/sessions.php")),
    ("langs/lib_php/native/streams.php", include_str!("native/streams.php")),
    ("langs/lib_php/native/strings.php", include_str!("native/strings.php")),
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
