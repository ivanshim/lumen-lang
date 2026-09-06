// Ported from examples/lumen/constructs/string_mixed.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let message = "value: ";
    let n = 42;
    println!("{}", message);
    println!("{}", n);
    if n == 42 {
        println!("correct");
    }
}
