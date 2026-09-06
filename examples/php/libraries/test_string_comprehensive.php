<?php
// Ported from examples/lumen/libraries/test_string_comprehensive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
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

function string_to_lower($s) {
    $result = "";
    $i = 0;
    while ($i < count($s)) {
        $result = $result . char_to_lower($s[$i]);
        $i = $i + 1;
    }
    return $result;
}

print("=== Text Processing Pipeline ===\n");
print("\n");
print("Password Strength Validator:\n");
function has_digit_char($s) {
    $digit_i = 0;
    while ($digit_i < count($s)) {
        if (is_digit($s[$digit_i])) {
            return true;
        }
        $digit_i = $digit_i + 1;
    }
    return false;
}

function has_alpha_char($s) {
    $alpha_i = 0;
    while ($alpha_i < count($s)) {
        if (is_alpha($s[$alpha_i])) {
            return true;
        }
        $alpha_i = $alpha_i + 1;
    }
    return false;
}

function has_upper_char($s) {
    $upper_i = 0;
    $c = "";
    $o = 0;
    while ($upper_i < count($s)) {
        $c = $s[$upper_i];
        $o = ord($c);
        if ($o >= ord("A") && $o <= ord("Z")) {
            return true;
        }
        $upper_i = $upper_i + 1;
    }
    return false;
}

function has_lower_char($s) {
    $lower_i = 0;
    $c = "";
    $o = 0;
    while ($lower_i < count($s)) {
        $c = $s[$lower_i];
        $o = ord($c);
        if ($o >= ord("a") && $o <= ord("z")) {
            return true;
        }
        $lower_i = $lower_i + 1;
    }
    return false;
}

function validate_password($pwd) {
    print(("  Password: '" . $pwd . "'") . "\n");
    if (count($pwd) < 8) {
        print("    WEAK: Too short (minimum 8 characters)\n");
        return false;
    }
    if (!has_digit_char($pwd)) {
        print("    WEAK: Must contain at least one digit\n");
        return false;
    }
    if (!has_alpha_char($pwd)) {
        print("    WEAK: Must contain at least one letter\n");
        return false;
    }
    if (!has_upper_char($pwd)) {
        print("    WEAK: Must contain at least one uppercase letter\n");
        return false;
    }
    if (!has_lower_char($pwd)) {
        print("    WEAK: Must contain at least one lowercase letter\n");
        return false;
    }
    print("    STRONG: All requirements met\n");
    return true;
}

$passwords = ["abc123", "PASSWORD123", "Pass123", "MyP@ssw0rd"];
$pwd_i = 0;
while ($pwd_i < count($passwords)) {
    validate_password($passwords[$pwd_i]);
    $pwd_i = $pwd_i + 1;
}
print("\n");
print("Extract and Sum Numbers from Text:\n");
function extract_all_numbers($s) {
    $numbers = [];
    $current_num = "";
    $extract_i = 0;
    $c = "";
    while ($extract_i < count($s)) {
        $c = $s[$extract_i];
        if (is_digit($c)) {
            $current_num = $current_num . $c;
        } elseif (strlen($current_num) > 0) {
            array_push($numbers, string_to_value($current_num));
            $current_num = "";
        }
        $extract_i = $extract_i + 1;
    }
    if (strlen($current_num) > 0) {
        array_push($numbers, string_to_value($current_num));
    }
    return $numbers;
}

function sum_numbers_in_text($s) {
    $nums = extract_all_numbers($s);
    $sum = 0;
    $sum_i = 0;
    while ($sum_i < count($nums)) {
        $sum = $sum + $nums[$sum_i];
        $sum_i = $sum_i + 1;
    }
    return $sum;
}

$texts = ["I have 5 apples and 3 oranges", "Order #123 total: 45 items", "No numbers here", "Year 2025 day 17"];
$text_i = 0;
$text = "";
$sum = 0;
while ($text_i < count($texts)) {
    $text = $texts[$text_i];
    $sum = sum_numbers_in_text($text);
    print(("  '" . $text . "'") . "\n");
    print(("    Sum: " . strval($sum)) . "\n");
    $text_i = $text_i + 1;
}
print("\n");
print("Normalize and Compare Strings:\n");
function normalize_string($s) {
    $lower = string_to_lower($s);
    $result = "";
    $normalize_i = 0;
    $c = "";
    while ($normalize_i < strlen($lower)) {
        $c = $lower[$normalize_i];
        if (is_alnum($c)) {
            $result = $result . $c;
        }
        $normalize_i = $normalize_i + 1;
    }
    return $result;
}

function strings_match_normalized($a, $b) {
    return normalize_string($a) == normalize_string($b);
}

$pairs = [["Hello World", "hello world"], ["Test-123", "TEST123"], ["Lumen Lang", "LumenLang"], ["Different", "Strings"]];
$pair_i = 0;
$a = "";
$b = "";
$match = false;
while ($pair_i < count($pairs)) {
    $a = $pairs[$pair_i][0];
    $b = $pairs[$pair_i][1];
    $match = strings_match_normalized($a, $b);
    print(("  '" . $a . "' vs '" . $b . "': " . strval($match)) . "\n");
    $pair_i = $pair_i + 1;
}
print("\n");
print("Generate Acronym from Phrase:\n");
function generate_acronym($phrase) {
    $words = [];
    $current_word = "";
    $acronym_i = 0;
    $c = "";
    while ($acronym_i < count($phrase)) {
        $c = $phrase[$acronym_i];
        if ($c == " ") {
            if (strlen($current_word) > 0) {
                array_push($words, $current_word);
                $current_word = "";
            }
        } else {
            $current_word = $current_word . $c;
        }
        $acronym_i = $acronym_i + 1;
    }
    if (strlen($current_word) > 0) {
        array_push($words, $current_word);
    }
    $acronym = "";
    $acronym_j = 0;
    $word = "";
    while ($acronym_j < count($words)) {
        $word = $words[$acronym_j];
        if (count($word) > 0) {
            $acronym = $acronym . char_to_upper($word[0]);
        }
        $acronym_j = $acronym_j + 1;
    }
    return $acronym;
}

$phrases = ["As Soon As Possible", "Frequently Asked Questions", "Light Amplification by Stimulated Emission of Radiation", "You Only Live Once"];
$phrase_i = 0;
$phrase = "";
$acronym = "";
while ($phrase_i < count($phrases)) {
    $phrase = $phrases[$phrase_i];
    $acronym = generate_acronym($phrase);
    print(("  '" . $phrase . "' -> " . $acronym) . "\n");
    $phrase_i = $phrase_i + 1;
}
print("\n");
print("Simple Caesar Cipher (letters only, shift by 3):\n");
function caesar_shift_char($c) {
    if (!is_alpha($c)) {
        return $c;
    }
    $o = ord($c);
    if ($o >= ord("a") && $o <= ord("z")) {
        $shifted = (($o - ord("a") + 3) % 26) + ord("a");
        return chr($shifted);
    } else {
        $shifted = (($o - ord("A") + 3) % 26) + ord("A");
        return chr($shifted);
    }
}

function caesar_encrypt($s) {
    $result = "";
    $caesar_i = 0;
    $c = "";
    $o = 0;
    $shifted = 0;
    while ($caesar_i < count($s)) {
        $c = $s[$caesar_i];
        if (!is_alpha($c)) {
            $result = $result . $c;
        } else {
            $o = ord($c);
            if ($o >= ord("a") && $o <= ord("z")) {
                $shifted = (($o - ord("a") + 3) % 26) + ord("a");
                $result = $result . chr($shifted);
            } else {
                $shifted = (($o - ord("A") + 3) % 26) + ord("A");
                $result = $result . chr($shifted);
            }
        }
        $caesar_i = $caesar_i + 1;
    }
    return $result;
}

$messages = ["hello world", "LUMEN LANG", "Test 123"];
$msg_i = 0;
$msg = "";
$encrypted = "";
while ($msg_i < count($messages)) {
    $msg = $messages[$msg_i];
    $encrypted = caesar_encrypt($msg);
    print(("  '" . $msg . "' -> '" . $encrypted . "'") . "\n");
    $msg_i = $msg_i + 1;
}
