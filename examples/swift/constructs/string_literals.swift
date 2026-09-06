// Ported from examples/lumen/constructs/string_literals.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let single1 = "hello"
let single2 = "world"
print(single1)
print(single2)
let double1 = "hello"
let double2 = "world"
print(double1)
print(double2)
print(single1 == double1)
print(single2 == double2)
let escaped_single = "can't"
print(escaped_single)
let literal_n = "text\\nmore"
print(literal_n)
let literal_t = "text\\tmore"
print(literal_t)
let escaped_double = "can't"
print(escaped_double)
let newline_test = "first\nsecond"
print(newline_test)
let tab_test = "col1\tcol2"
print(tab_test)
let escaped_quote = "He said \"hello\""
print(escaped_quote)
let escaped_backslash = "path\\to\\file"
print(escaped_backslash)
let no_interp = "${name}"
print(no_interp)
let no_interp2 = "{count}"
print(no_interp2)
let x = "10"
let no_interp3 = "The answer is ${x}"
print(no_interp3)
