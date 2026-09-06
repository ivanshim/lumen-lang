<?php
// Ported from examples/lumen/constructs/type_annotations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: Type Annotations on Variables\n");
$x = 42;
print("let x: number = 42\n");
print($x . "\n");
$message = "Hello, World";
print("let message: string = \n");
print($message . "\n");
$flag = true;
print("let flag: boolean = true\n");
print($flag . "\n");
$empty = null;
print("let empty: null = null\n");
print($empty . "\n");
function add($a, $b) {
    return $a + $b;
}

print("add(5, 3):\n");
print(add(5, 3) . "\n");
function greet($name) {
    return "Hello, " + $name;
}

print("greet(\"Alice\"):\n");
print(greet("Alice") . "\n");
function process($x, $y) {
    $result = $x * 2 + $y;
    return $result;
}

print("process(10, 5):\n");
print(process(10, 5) . "\n");
