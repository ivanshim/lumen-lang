// Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn add_numbers(a: i64, b: i64) -> i64 {
    return a + b;
}

fn main() {
    println!("Test: Comments Support");
    let x = 42;
    println!("{}", x);
    let result = x * 2;
    println!("{}", result);
    let value = add_numbers(10, 20);
    println!("{}", value);
    if value > 20 {
        println!("Value is greater than 20");
    } else {
        println!("Value is 20 or less");
    }
    let mut counter = 0;
    while counter < 3 {
        println!("{}", counter);
        counter = counter + 1;
    }
    println!("Done testing comments");
}
