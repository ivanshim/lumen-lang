// Ported from examples/lumen/constructs/scope_leak.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let mut y = 100;
    println!("{}", y);
    if true {
        y = 50;
        println!("{}", y);
    }
    println!("{}", y);
}
