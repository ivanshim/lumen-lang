// Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn test_let() -> i64 {
    let a = 42;
    let mut b = 10;
    b = 50;
    return a + b;
}

fn main() {
    println!("Test: let and let mut Keywords");
    let mut x = 10;
    println!("let x = 10");
    println!("{}", x);
    let mut y = 5;
    println!("let mut y = 5");
    println!("{}", y);
    y = 20;
    println!("After y = 20:");
    println!("{}", y);
    x = 100;
    println!("After let x = 100 (shadowing):");
    println!("{}", x);
    let result = x + y;
    println!("let result = x + y");
    println!("{}", result);
    println!("test_let():");
    println!("{}", test_let());
}
