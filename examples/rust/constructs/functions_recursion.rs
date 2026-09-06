// Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn factorial(n: i64) -> i64 {
    if n <= 1 {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

fn countdown(n: i64) -> i64 {
    if n <= 0 {
        println!("Done");
    } else {
        println!("{}", n);
        return countdown(n - 1);
    }
}

fn main() {
    println!("Test: Recursion");
    println!("Factorial of 5:");
    println!("{}", factorial(5));
    println!("Countdown from 3:");
    countdown(3);
}
