// Ported from examples/lumen/constructs/scope_if.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let mut x = 10;
    if true {
        x = 20;
        println!("{}", x);
    } else {
        x = 30;
        println!("{}", x);
    }
    println!("{}", x);
}
