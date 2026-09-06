// Ported from examples/lumen/exponentiation_naive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    let mut j;
    let base = 7;
    let exp = 100;
    let mod = 1000000007;
    let iterations = 100;
    println!("Naive exponentiation benchmark");
    print!("base = ");
    println!("{}", base);
    print!("exp  = ");
    println!("{}", exp);
    print!("mod  = ");
    println!("{}", mod);
    print!("iterations = ");
    println!("{}", iterations);
    println!("");
    println!("Running naive exponentiation...");
    let mut result = 0;
    let mut i = 0;
    while i < iterations {
        result = 1;
        j = 0;
        while j < exp {
            result = result * base;
            j = j + 1;
        }
        result = result % mod;
        i = i + 1;
    }
    print!("Result: ");
    println!("{}", result);
    println!("Done!");
}
