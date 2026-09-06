# Ported from examples/lumen/constructs/string_literals.lm by scripts/port_examples.py; edit the Lumen original, not this file.
single1 = "hello"
single2 = "world"
puts(single1)
puts(single2)
double1 = "hello"
double2 = "world"
puts(double1)
puts(double2)
puts(single1 == double1)
puts(single2 == double2)
escaped_single = "can't"
puts(escaped_single)
literal_n = "text\\nmore"
puts(literal_n)
literal_t = "text\\tmore"
puts(literal_t)
escaped_double = "can't"
puts(escaped_double)
newline_test = "first\nsecond"
puts(newline_test)
tab_test = "col1\tcol2"
puts(tab_test)
escaped_quote = "He said \"hello\""
puts(escaped_quote)
escaped_backslash = "path\\to\\file"
puts(escaped_backslash)
no_interp = "${name}"
puts(no_interp)
no_interp2 = "{count}"
puts(no_interp2)
x = "10"
no_interp3 = "The answer is ${x}"
puts(no_interp3)
