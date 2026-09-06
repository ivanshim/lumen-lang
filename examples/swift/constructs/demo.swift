// Ported from examples/lumen/constructs/demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print(1 + 2 * 3)
let x = 0
let y = 5
if x < y && y == 5 {
    print(100)
} else {
    print(200)
}
var i = 0
var sum = 0
while i < 10 {
    if i == 5 {
        i = i + 1
        continue
    }
    if i == 8 {
        break
    }
    sum = sum + i
    print(sum)
    i = i + 1
}
print(sum)
print(true)
print(false)
print(!false)
print(-10 + 3)
