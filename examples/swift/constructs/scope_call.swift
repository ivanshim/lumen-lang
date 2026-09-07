// Ported from examples/lumen/constructs/scope_call.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var k = 1
func show() {
    print(k)
}

func caller() {
    let k = 5
    show()
    print(k)
}

caller()
print(k)
show()
