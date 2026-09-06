// Ported from examples/lumen/test_scope_leak_fix.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn count_to_three() -> i64 {
    let mut i = 0;
    while i < 3 {
        i = i + 1;
    }
    return i;
}

fn main() {
    println!("{}", "First call: " + count_to_three().to_string());
    println!("{}", "Second call: " + count_to_three().to_string());
    println!("{}", "Third call: " + count_to_three().to_string());
    println!("All calls completed successfully!");
}
