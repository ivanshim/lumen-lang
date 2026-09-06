<?php
// Ported from examples/lumen/constructs/string_equality.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$x = "hello";
$y = "hello";
$z = "world";
print(($x == $y) . "\n");
print(($x == $z) . "\n");
print(($x != $z) . "\n");
