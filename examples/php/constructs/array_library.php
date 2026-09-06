<?php
// Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function array_concat($a, $b) {
    $out = [];
    $i = 0;
    while ($i < count($a)) {
        array_push($out, $a[$i]);
        $i = $i + 1;
    }
    $i = 0;
    while ($i < count($b)) {
        array_push($out, $b[$i]);
        $i = $i + 1;
    }
    return $out;
}

function array_slice($a, $start, $stop) {
    $out = [];
    $i = $start;
    while ($i < $stop) {
        array_push($out, $a[$i]);
        $i = $i + 1;
    }
    return $out;
}

function array_index_of($a, $x) {
    $i = 0;
    while ($i < count($a)) {
        if ($a[$i] == $x) {
            return $i;
        }
        $i = $i + 1;
    }
    return -1;
}

function array_contains($a, $x) {
    return array_index_of($a, $x) >= 0;
}

function array_reverse($a) {
    $out = [];
    $i = count($a);
    while ($i > 0) {
        $i = $i - 1;
        array_push($out, $a[$i]);
    }
    return $out;
}

$a = [1, 2, 3];
$b = [4, 5];
$both = array_concat($a, $b);
print($both . "\n");
print(array_slice($both, 1, 4) . "\n");
print(array_index_of($b, 5) . "\n");
print(array_index_of($b, 9) . "\n");
print(array_contains($a, 2) . "\n");
print(array_contains($a, 7) . "\n");
print(array_reverse($both) . "\n");
print(count(array_reverse([])) . "\n");
