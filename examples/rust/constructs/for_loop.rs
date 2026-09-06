// Ported from examples/lumen/constructs/for_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    for i in 0..10 {
        print!("{}", i);
        if i < 9 {
            print!(", ");
        }
    }
    println!("");
}
