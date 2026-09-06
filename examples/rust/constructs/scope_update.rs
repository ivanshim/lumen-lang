// Ported from examples/lumen/constructs/scope_update.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let mut counter = 0;
    if true {
        counter = counter + 1;
    }
    if true {
        counter = counter + 1;
    }
    println!("{}", counter);
}
