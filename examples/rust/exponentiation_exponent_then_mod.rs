// Ported from examples/lumen/exponentiation_exponent_then_mod.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn mod_pow(base: i64, exp: i64, m: i64) -> i64 {
    let mut result = 1;
    base = base % m;
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % m;
        }
        exp = exp / 2;
        base = (base * base) % m;
    }
    return result;
}

fn main() {
    let base = 7;
    let exp = 100;
    let mod = 1000000007;
    let iterations = 100;
    println!("Fast modular exponentiation benchmark");
    print!("base = ");
    println!("{}", base);
    print!("exp  = ");
    println!("{}", exp);
    print!("mod  = ");
    println!("{}", mod);
    print!("iterations = ");
    println!("{}", iterations);
    println!("");
    println!("Running mod_pow...");
    let mut result = 0;
    let mut i = 0;
    while i < iterations {
        result = mod_pow(base, exp, mod);
        i = i + 1;
    }
    print!("Result: ");
    println!("{}", result);
    println!("Done!");
}
