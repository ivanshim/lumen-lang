// The Lumen library file lib_lumen/string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function char_at_or_null($s, $index) {
    if ($index < 0 || $index >= count($s)) {
        return null;
    }
    return $s[$index];
}

function substring($s, $from_start, $to_end) {
    $index = $from_start;
    $out = "";
    while ($index < $to_end) {
        $out = $out . $s[$index];
        $index = $index + 1;
    }
    return $out;
}

function substring_end($s, $from_here) {
    return substring($s, $from_here, count($s));
}

function substring_start($s, $to_here) {
    return substring($s, 0, $to_here);
}

function starts_with($s, $prefix) {
    return count($prefix) <= count($s) && substring($s, 0, count($prefix)) == $prefix;
}

function ends_with($s, $suffix) {
    return count($suffix) <= count($s) && substring($s, count($s) - count($suffix), count($s)) == $suffix;
}

function repeat_string($s, $repetitions) {
    $out = "";
    $i = 0;
    while ($i < $repetitions) {
        $out = $out . $s;
        $i = $i + 1;
    }
    return $out;
}

function join_strings($arr, $separator) {
    $out = "";
    $n = count($arr);
    $i = 0;
    while ($i < $n) {
        if ($i > 0) {
            $out = $out . $separator;
        }
        $out = $out . $arr[$i];
        $i = $i + 1;
    }
    return $out;
}

function index_of($s, $needle) {
    $n = count($needle);
    $i = 0;
    while ($i + $n <= count($s)) {
        if (substring($s, $i, $i + $n) == $needle) {
            return $i;
        }
        $i = $i + 1;
    }
    return -1;
}

function has_substring($s, $needle) {
    return index_of($s, $needle) >= 0;
}
