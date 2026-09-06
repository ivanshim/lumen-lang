// Ported from examples/lumen/constructs/array_mutations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
fn main() {
    println!("=== Array Features Test ===");
    let arr = [10, 20, 30];
    print!("Array: ");
    println!("{}", arr);
    print!("arr[0] = ");
    println!("{}", arr[0]);
    let arr2 = [1, 2, 3];
    arr2[1] = 999;
    print!("After arr2[1]=999: ");
    println!("{}", arr2);
    let arr3 = [];
    arr3.push(100);
    arr3.push(200);
    print!("After push: ");
    println!("{}", arr3);
    println!("Done!");
}
