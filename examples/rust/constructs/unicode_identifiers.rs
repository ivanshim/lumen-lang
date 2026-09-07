// Ported from examples/lumen/constructs/unicode_identifiers.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn größe(x: i64) -> i64 {
    return x + 1;
}

fn main() {
    let café = 3;
    let π = 3.14159;
    let 数 = café * 2;
    println!("{}", café);
    println!("{}", π);
    println!("{}", 数);
    println!("{}", größe(数));
}
