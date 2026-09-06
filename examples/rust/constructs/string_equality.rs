// Ported from examples/lumen/constructs/string_equality.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let x = "hello";
    let y = "hello";
    let z = "world";
    println!("{}", x == y);
    println!("{}", x == z);
    println!("{}", x != z);
}
