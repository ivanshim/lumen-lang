// Ported from examples/lumen/constructs/for_loop_control.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    for i in 0..15 {
        if i == 10 {
            break;
        }
        print!("{}", i);
        if i < 9 {
            print!(", ");
        }
    }
    println!("");
}
