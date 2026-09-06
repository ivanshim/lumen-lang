// Ported from examples/lumen/constructs/array_mutations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
console.log("=== Array Features Test ===");
const arr = [10, 20, 30];
process.stdout.write("Array: ");
console.log(arr);
process.stdout.write("arr[0] = ");
console.log(arr[0]);
const arr2 = [1, 2, 3];
arr2[1] = 999;
process.stdout.write("After arr2[1]=999: ");
console.log(arr2);
const arr3 = [];
arr3.push(100);
arr3.push(200);
process.stdout.write("After push: ");
console.log(arr3);
console.log("Done!");
