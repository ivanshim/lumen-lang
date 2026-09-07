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

function digits_to_base_value($s, $i, $base) {
    $start = $i;
    $value = 0;
    $scale = 1;
    while ($i < count($s)) {
        $c = $s[$i];
        $d = character_to_value($c);
        if ($d < 0 || $d >= $base) {
            break;
        }
        $value = $value * $base + $d;
        $scale = $scale * $base;
        $i = $i + 1;
    }
    if ($i == $start) {
        exit("expected digit");
    }
    return [$value, $scale, $i];
}

function numeric_literal_to_value($s, $i) {
    $start = $i;
    $base_prefix = 0;
    while ($i < count($s)) {
        $c = $s[$i];
        if (!is_digit($c)) {
            break;
        }
        $base_prefix = $base_prefix * 10 + (ord($c) - ord("0"));
        $i = $i + 1;
    }
    if ($i == $start) {
        return [0, $start];
    }
    $base = 10;
    $value = $base_prefix;
    if ($i < count($s) && $s[$i] == "@") {
        $base = $base_prefix;
        if ($base < 2 || $base > 36) {
            exit("invalid base");
        }
        $i = $i + 1;
        $r = digits_to_base_value($s, $i, $base);
        $value = $r[0];
        $i = $r[2];
    }
    if ($i < count($s) && $s[$i] == ".") {
        $i = $i + 1;
        $r2 = digits_to_base_value($s, $i, $base);
        $frac_val = $r2[0];
        $frac_scale = $r2[1];
        $i = $r2[2];
        if ($frac_scale > 1) {
            $value = $value + $frac_val / $frac_scale;
        }
    }
    return [$value, $i];
}

function string_to_value($s) {
    if (count($s) == 0) {
        return 0;
    }
    $i = 0;
    $r = numeric_literal_to_value($s, $i);
    $num = $r[0];
    $i = $r[1];
    if ($i < count($s) && $s[$i] == "/") {
        $i = $i + 1;
        if ($i == count($s)) {
            return $s;
        }
        $r2 = numeric_literal_to_value($s, $i);
        $denom = $r2[0];
        $i = $r2[1];
        if ($i != count($s)) {
            return $s;
        }
        return $num / $denom;
    }
    if ($i == count($s)) {
        return $num;
    }
    return $s;
}
