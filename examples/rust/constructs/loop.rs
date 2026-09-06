// Ported from examples/lumen/constructs/loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let mut x = 0;
    while x < 10 {
        print!("{}", x);
        if x < 9 {
            print!(", ");
        }
        x = x + 1;
    }
    println!("");
}
