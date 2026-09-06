// Ported from examples/lumen/constructs/scope_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let mut i = 0;
    let mut sum = 0;
    while i < 5 {
        sum = sum + i;
        println!("{}", sum);
        i = i + 1;
    }
    println!("{}", i);
    println!("{}", sum);
}
