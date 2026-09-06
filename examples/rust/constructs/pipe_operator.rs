// Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn double(x: i64) -> i64 {
    return x * 2;
}

fn add_one(x: i64) -> i64 {
    return x + 1;
}

fn square(x: i64) -> i64 {
    return x * x;
}

fn multiply(a: i64, b: i64) -> i64 {
    return a * b;
}

fn main() {
    println!("Test: Pipe Operator");
    println!("Without pipe: square(add_one(double(5)))");
    println!("{}", square(add_one(double(5))));
    println!("With pipe: 5 |> double() |> add_one() |> square()");
    let result = square(add_one(double(5)));
    println!("{}", result);
    println!("10 |> double():");
    println!("{}", double(10));
    println!("3 |> double():");
    let x = double(3);
    println!("{}", multiply(x, 2));
}
