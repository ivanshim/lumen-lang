// Ported from examples/lumen/constructs/scope_call.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let k = 1;
function show() {
    console.log(k);
}

function caller() {
    const k = 5;
    show();
    console.log(k);
}

caller();
console.log(k);
show();
