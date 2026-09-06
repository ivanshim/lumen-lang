<?php
// Ported from examples/lumen/constructs/unicode_identifiers.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$café = 3;
$π = 22 / 7;
$数 = $café * 2;
function größe($x) {
    return $x + 1;
}

print($café . "\n");
print($π . "\n");
print($数 . "\n");
print(größe($数) . "\n");
