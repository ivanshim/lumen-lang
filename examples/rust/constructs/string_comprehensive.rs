// Ported from examples/lumen/constructs/string_comprehensive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let a = "alpha";
    let b = "beta";
    println!("{}", a);
    println!("{}", b);
    if a == "alpha" {
        println!("a is alpha");
    }
    if a != b {
        println!("a and b are different");
    }
    let x = 10;
    let y = "number";
    println!("{}", x);
    println!("{}", y);
}
