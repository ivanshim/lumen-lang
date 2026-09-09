// Modules kept as source until a program asks for them.
pub static MODULES: &[(&str, &str)] = &[
    ("functools", include_str!("functools.py")),
    ("itertools", include_str!("itertools.py")),
    ("math", include_str!("math.py")),
    ("operator", include_str!("operator.py")),
    ("sys", include_str!("sys.py")),
    ("test", include_str!("test/__init__.py")),
    ("unittest", include_str!("unittest.py")),
];
