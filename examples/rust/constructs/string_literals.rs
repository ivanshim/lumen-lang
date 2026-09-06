// Ported from examples/lumen/constructs/string_literals.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let single1 = "hello";
    let single2 = "world";
    println!("{}", single1);
    println!("{}", single2);
    let double1 = "hello";
    let double2 = "world";
    println!("{}", double1);
    println!("{}", double2);
    println!("{}", single1 == double1);
    println!("{}", single2 == double2);
    let escaped_single = "can't";
    println!("{}", escaped_single);
    let literal_n = "text\\nmore";
    println!("{}", literal_n);
    let literal_t = "text\\tmore";
    println!("{}", literal_t);
    let escaped_double = "can't";
    println!("{}", escaped_double);
    let newline_test = "first\nsecond";
    println!("{}", newline_test);
    let tab_test = "col1\tcol2";
    println!("{}", tab_test);
    let escaped_quote = "He said \"hello\"";
    println!("{}", escaped_quote);
    let escaped_backslash = "path\\to\\file";
    println!("{}", escaped_backslash);
    let no_interp = "${name}";
    println!("{}", no_interp);
    let no_interp2 = "{count}";
    println!("{}", no_interp2);
    let x = "10";
    let no_interp3 = "The answer is ${x}";
    println!("{}", no_interp3);
}
