// Ported from examples/lumen/constructs/none_type.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function no_return() {
    console.log("This function returns null implicitly");
}

function explicit_null() {
    console.log("Returning null explicitly");
    return null;
}

function conditional_null(x) {
    if (x < 0) {
        return null;
    } else {
        return x * 2;
    }
}

console.log("Test: null Type");
console.log("Calling no_return():");
const result1 = no_return();
console.log(result1);
console.log("Calling explicit_null():");
const result2 = explicit_null();
console.log(result2);
console.log("conditional_null(5):");
console.log(conditional_null(5));
console.log("conditional_null(-3):");
console.log(conditional_null(-3));
const x = null;
console.log("let x = null:");
console.log(x);
function check_value(val) {
    if (val === null) {
        console.log("Value is null");
    } else {
        console.log("Value is not null");
    }
}

check_value(null);
check_value(42);
check_value("hello");
