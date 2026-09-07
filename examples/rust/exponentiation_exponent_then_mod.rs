// Ported from examples/lumen/exponentiation_exponent_then_mod.lm by scripts/port_examples.py; edit the Lumen original, not this file.
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
