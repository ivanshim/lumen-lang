# Ported from examples/lumen/constructs/scope_call.lm by scripts/port_examples.py; edit the Lumen original, not this file.
k = 1
def show():
    print(k)

def caller():
    k = 5
    show()
    print(k)

caller()
print(k)
show()
