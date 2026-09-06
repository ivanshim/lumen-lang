// Ported from examples/lumen/constructs/until_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    print!("Until loop ascending (0-9): ");
    let mut i = 0;
    while !(i >= 10) {
        print!("{}", i);
        if i < 9 {
            print!(", ");
        }
        i = i + 1;
    }
    println!("");
    print!("Until loop descending (15-6): ");
    let mut x = 15;
    while !(x <= 5) {
        print!("{}", x);
        if x > 6 {
            print!(", ");
        }
        x = x - 1;
    }
    println!("");
}
