// Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    println!("{}", 1);
    let mut x = 10;
    println!("{}", x);
    if true {
        x = 20;
        println!("{}", x);
    }
    println!("{}", x);
}
