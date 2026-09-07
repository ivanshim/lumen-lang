// The Lumen library file lib_lumen/array.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

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
