// Ported from examples/lumen/constructs/write_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    print!("Hello");
    print!(" ");
    print!("World");
    print!("!");
    println!("");
    print!("Numbers: ");
    let mut i = 1;
    while i <= 5 {
        print!("{}", i);
        if i < 5 {
            print!(", ");
        }
        i = i + 1;
    }
    println!("");
}
