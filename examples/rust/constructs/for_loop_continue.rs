// Ported from examples/lumen/constructs/for_loop_continue.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    for i in 0..11 {
        if i == 5 {
            continue;
        }
        print!("{}", i);
        if i < 10 {
            print!(", ");
        }
    }
    println!("");
}
