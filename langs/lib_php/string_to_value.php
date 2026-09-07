// The Lumen library file langs/lib_lumen/string_to_value.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function character_to_value($c) {
    if (is_digit($c)) {
        return ord($c) - ord("0");
    }
    $code = ord($c);
    if ($code >= ord("A") && $code <= ord("Z")) {
        return $code - ord("A") + 10;
    }
    if ($code >= ord("a") && $code <= ord("z")) {
        return $code - ord("a") + 10;
    }
    return -1;
}
