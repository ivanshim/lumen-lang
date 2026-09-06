// Ported from examples/lumen/constructs/functions_basic.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn square(x: i64) -> i64 {
    return x * x;
}

fn add(a: i64, b: i64) -> i64 {
    return a + b;
}

fn greet(name: String) -> String {
    return "Hello, " + name;
}

fn get_constant() -> i64 {
    return 42;
}

fn compute(x: i64, y: i64) -> i64 {
    let sum = x + y;
    let product = x * y;
    return sum + product;
}

fn main() {
    println!("Test: Basic Functions");
    println!("{}", square(5));
    println!("{}", add(10, 20));
    println!("{}", greet("Lumen"));
    println!("{}", get_constant());
    println!("{}", compute(3, 4));
}
