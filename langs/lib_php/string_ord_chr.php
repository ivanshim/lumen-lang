// The Lumen library file langs/lib_lumen/string_ord_chr.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function is_ascii($c) {
    return ord($c) < 128;
}

function is_digit($c) {
    $o = ord($c);
    return $o >= ord("0") && $o <= ord("9");
}

function is_alpha($c) {
    $o = ord($c);
    return ($o >= ord("A") && $o <= ord("Z")) || ($o >= ord("a") && $o <= ord("z"));
}

function is_alnum($c) {
    return is_alpha($c) || is_digit($c);
}

function char_to_upper($c) {
    $o = ord($c);
    if ($o >= ord("a") && $o <= ord("z")) {
        return chr($o - 32);
    } else {
        return $c;
    }
}

function char_to_lower($c) {
    $o = ord($c);
    if ($o >= ord("A") && $o <= ord("Z")) {
        return chr($o + 32);
    } else {
        return $c;
    }
}

function string_to_upper($s) {
    $result = "";
    $i = 0;
    while ($i < count($s)) {
        $result = $result . char_to_upper($s[$i]);
        $i = $i + 1;
    }
    return $result;
}

function string_to_lower($s) {
    $result = "";
    $i = 0;
    while ($i < count($s)) {
        $result = $result . char_to_lower($s[$i]);
        $i = $i + 1;
    }
    return $result;
}

function reverse_characters($s) {
    $result = "";
    $index = count($s) - 1;
    while ($index >= 0) {
        $result = $result . $s[$index];
        $index = $index - 1;
    }
    return $result;
}

function capitalize_first_word($s) {
    $result = "";
    $i = 0;
    $done = false;
    while ($i < count($s)) {
        $c = $s[$i];
        if (!$done && is_alpha($c)) {
            $result = $result . char_to_upper($c);
            $done = true;
        } else {
            $result = $result . $c;
        }
        $i = $i + 1;
    }
    return $result;
}

function capitalize_words($s) {
    $result = "";
    $i = 0;
    $at_word_start = true;
    while ($i < count($s)) {
        $c = $s[$i];
        if (is_alpha($c)) {
            if ($at_word_start) {
                $result = $result . char_to_upper($c);
                $at_word_start = false;
            } else {
                $result = $result . $c;
            }
        } else {
            $result = $result . $c;
            $at_word_start = true;
        }
        $i = $i + 1;
    }
    return $result;
}

function is_whitespace($c) {
    $o = ord($c);
    return $o == 32 || $o == 9 || $o == 10 || $o == 13;
}

function trim_start($s) {
    $i = 0;
    while ($i < count($s) && is_whitespace($s[$i])) {
        $i = $i + 1;
    }
    return substring_end($s, $i);
}

function trim_end($s) {
    $i = count($s) - 1;
    while ($i >= 0 && is_whitespace($s[$i])) {
        $i = $i - 1;
    }
    return substring($s, 0, $i + 1);
}

function trim($s) {
    return trim_start(trim_end($s));
}

function is_alpha_string($s) {
    if (count($s) == 0) {
        return false;
    }
    $i = 0;
    while ($i < count($s)) {
        if (!is_alpha($s[$i])) {
            return false;
        }
        $i = $i + 1;
    }
    return true;
}
