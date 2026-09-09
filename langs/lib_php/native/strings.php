// What PHP does with text beyond the kernel's own words, and the few
// odds and ends that go with it. Written in PHP because it is PHP's and
// not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// Which end of a piece of text is filled out.
define("STR_PAD_RIGHT", 1);
define("STR_PAD_LEFT", 0);
define("STR_PAD_BOTH", 2);

// The characters a list stands for. Two dots between two characters
// stand for every character from the one to the other.
function __characters_of($list) {
    $held = array();
    $at = 0;
    while ($at < strlen($list)) {
        if ($at + 3 < strlen($list) && $list[$at + 1] === '.' && $list[$at + 2] === '.') {
            $from = ord($list[$at]);
            $to = ord($list[$at + 3]);
            while ($from <= $to) { $held[] = chr($from); $from = $from + 1; }
            $at = $at + 4;
            continue;
        }
        $held[] = $list[$at];
        $at = $at + 1;
    }
    return $held;
}

// Whatever the list names taken off the front of a piece of text, off
// the end of it, or off both. The list left out means the blank
// characters, which is what taking whitespace off means.
function __pared($text, $characters, $front, $back) {
    $text = (string) $text;
    $wanted = $characters === null ? array(" ", "\t", "\n", "\r", "\0", "\x0B") : __characters_of($characters);
    $from = 0;
    $to = strlen($text);
    if ($front) {
        while ($from < $to && in_array($text[$from], $wanted, true)) { $from = $from + 1; }
    }
    if ($back) {
        while ($to > $from && in_array($text[$to - 1], $wanted, true)) { $to = $to - 1; }
    }
    return substr($text, $from, $to - $from);
}
function trim($string, $characters = null) { return __pared($string, $characters, true, true); }
function ltrim($string, $characters = null) { return __pared($string, $characters, true, false); }
function rtrim($string, $characters = null) { return __pared($string, $characters, false, true); }
function chop($string, $characters = null) { return __pared($string, $characters, false, true); }

// The greatest of what is given, and the least. One array given stands
// for its own items; anything else, for the arguments themselves.
function max($value) {
    $items = func_num_args() === 1 && is_array($value) ? array_values($value) : func_get_args();
    if (count($items) === 0) { throw new ValueError('max(): Argument #1 ($value) must contain at least one element'); }
    $held = $items[0];
    foreach ($items as $item) { if ($item > $held) { $held = $item; } }
    return $held;
}
function min($value) {
    $items = func_num_args() === 1 && is_array($value) ? array_values($value) : func_get_args();
    if (count($items) === 0) { throw new ValueError('min(): Argument #1 ($value) must contain at least one element'); }
    $held = $items[0];
    foreach ($items as $item) { if ($item < $held) { $held = $item; } }
    return $held;
}

// How many times one piece of text stands inside another, counted
// without overlapping.
function substr_count($haystack, $needle, $offset = 0, $length = null) {
    $where = $length === null ? substr($haystack, $offset) : substr($haystack, $offset, $length);
    if ($needle === "") { throw new ValueError('substr_count(): Argument #2 ($needle) cannot be empty'); }
    $held = 0;
    $at = 0;
    while ($at + strlen($needle) <= strlen($where)) {
        if (substr($where, $at, strlen($needle)) === $needle) { $held = $held + 1; $at = $at + strlen($needle); }
        else { $at = $at + 1; }
    }
    return $held;
}

// The first letter made small.
function lcfirst($string) {
    if ($string === "") { return $string; }
    return strtolower($string[0]) . substr($string, 1);
}

// Text broken into lines no wider than a width. Words wider than the
// width are left whole unless the caller says to cut them.
function wordwrap($string, $width = 75, $break = "\n", $cut_long_words = false) {
    $lines = array();
    $line = "";
    foreach (explode(" ", $string) as $word) {
        while ($cut_long_words && strlen($word) > $width) {
            $room = $line === "" ? $width : $width - strlen($line) - 1;
            if ($room < 1) { $lines[] = $line; $line = ""; continue; }
            $piece = substr($word, 0, $room);
            $lines[] = $line === "" ? $piece : $line . " " . $piece;
            $line = "";
            $word = substr($word, $room);
        }
        if ($line === "") { $line = $word; }
        else if (strlen($line) + 1 + strlen($word) <= $width) { $line = $line . " " . $word; }
        else { $lines[] = $line; $line = $word; }
    }
    $lines[] = $line;
    return implode($break, $lines);
}

// A line break written out as one, with the break itself kept.
function nl2br($string, $use_xhtml = true) {
    $mark = $use_xhtml ? "<br />" : "<br>";
    $out = "";
    $at = 0;
    while ($at < strlen($string)) {
        $c = $string[$at];
        if ($c === "\r" && $at + 1 < strlen($string) && $string[$at + 1] === "\n") {
            $out = $out . $mark . "\r\n";
            $at = $at + 2;
            continue;
        }
        if ($c === "\n" && $at + 1 < strlen($string) && $string[$at + 1] === "\r") {
            $out = $out . $mark . "\n\r";
            $at = $at + 2;
            continue;
        }
        if ($c === "\n" || $c === "\r") { $out = $out . $mark . $c; }
        else { $out = $out . $c; }
        $at = $at + 1;
    }
    return $out;
}

// How many words a piece of text holds, or the words themselves. A word
// is a run of letters, with apostrophes and dashes inside it.
function str_word_count($string, $format = 0, $characters = null) {
    $extra = $characters === null ? array() : __characters_of($characters);
    $words = array();
    $places = array();
    $word = "";
    $began = 0;
    $at = 0;
    while ($at <= strlen($string)) {
        $c = $at < strlen($string) ? $string[$at] : "";
        $part = $c !== "" && (__is_letter($c) || $c === "'" || $c === "-" || in_array($c, $extra, true));
        if ($part) {
            if ($word === "") { $began = $at; }
            $word = $word . $c;
        } else if ($word !== "") {
            $words[] = $word;
            $places[] = $began;
            $word = "";
        }
        $at = $at + 1;
    }
    if ($format === 1) { return $words; }
    if ($format === 2) {
        $out = array();
        $i = 0;
        while ($i < count($words)) { $out[$places[$i]] = $words[$i]; $i = $i + 1; }
        return $out;
    }
    return count($words);
}
function __is_letter($c) { return ($c >= 'a' && $c <= 'z') || ($c >= 'A' && $c <= 'Z'); }

// How many single changes turn one piece of text into another.
function levenshtein($string1, $string2) {
    $a = (string) $string1;
    $b = (string) $string2;
    $row = array();
    $j = 0;
    while ($j <= strlen($b)) { $row[] = $j; $j = $j + 1; }
    $i = 0;
    while ($i < strlen($a)) {
        $next = array($i + 1);
        $j = 0;
        while ($j < strlen($b)) {
            $same = $a[$i] === $b[$j] ? 0 : 1;
            $one = $row[$j + 1] + 1;
            $two = $next[$j] + 1;
            $three = $row[$j] + $same;
            $least = $one < $two ? $one : $two;
            if ($three < $least) { $least = $three; }
            $next[] = $least;
            $j = $j + 1;
        }
        $row = $next;
        $i = $i + 1;
    }
    return $row[strlen($b)];
}

// How many characters two pieces of text have in common, found by
// taking the longest run they share and going on either side of it.
function similar_text($string1, $string2, &$percent = null) {
    $held = __alike($string1, $string2);
    $total = strlen($string1) + strlen($string2);
    $percent = $total === 0 ? 0.0 : $held * 2.0 * 100.0 / $total;
    return $held;
}
function __alike($a, $b) {
    $best = 0;
    $at_a = 0;
    $at_b = 0;
    $i = 0;
    while ($i < strlen($a)) {
        $j = 0;
        while ($j < strlen($b)) {
            $run = 0;
            while ($i + $run < strlen($a) && $j + $run < strlen($b) && $a[$i + $run] === $b[$j + $run]) { $run = $run + 1; }
            if ($run > $best) { $best = $run; $at_a = $i; $at_b = $j; }
            $j = $j + 1;
        }
        $i = $i + 1;
    }
    if ($best === 0) { return 0; }
    $before = __alike(substr($a, 0, $at_a), substr($b, 0, $at_b));
    $after = __alike(substr($a, $at_a + $best), substr($b, $at_b + $best));
    return $best + $before + $after;
}

// Quotes and backslashes written so that they may be read back as
// themselves.
function addslashes($string) {
    $out = "";
    $at = 0;
    while ($at < strlen($string)) {
        $c = $string[$at];
        if ($c === "'" || $c === '"' || $c === "\\" || $c === "\0") {
            $out = $out . "\\" . ($c === "\0" ? "0" : $c);
        } else { $out = $out . $c; }
        $at = $at + 1;
    }
    return $out;
}
function stripslashes($string) {
    $out = "";
    $at = 0;
    while ($at < strlen($string)) {
        $c = $string[$at];
        if ($c === "\\" && $at + 1 < strlen($string)) {
            $next = $string[$at + 1];
            $out = $out . ($next === "0" ? "\0" : $next);
            $at = $at + 2;
            continue;
        }
        if ($c !== "\\") { $out = $out . $c; }
        $at = $at + 1;
    }
    return $out;
}

// Everything between a `<` and the `>` that closes it taken out.
function strip_tags($string, $allowed_tags = null) {
    $out = "";
    $inside = false;
    $at = 0;
    while ($at < strlen($string)) {
        $c = $string[$at];
        if ($c === "<") { $inside = true; }
        else if ($c === ">") { $inside = false; }
        else if (!$inside) { $out = $out . $c; }
        $at = $at + 1;
    }
    return $out;
}

// Bytes written as the sixty-four characters that may be sent
// anywhere, and read back again.
function base64_encode($string) {
    $alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    $out = "";
    $at = 0;
    while ($at < strlen($string)) {
        $left = strlen($string) - $at;
        $one = ord($string[$at]);
        $two = $left > 1 ? ord($string[$at + 1]) : 0;
        $three = $left > 2 ? ord($string[$at + 2]) : 0;
        $out = $out . $alphabet[($one >> 2) & 63];
        $out = $out . $alphabet[(($one << 4) | ($two >> 4)) & 63];
        $out = $out . ($left > 1 ? $alphabet[(($two << 2) | ($three >> 6)) & 63] : "=");
        $out = $out . ($left > 2 ? $alphabet[$three & 63] : "=");
        $at = $at + 3;
    }
    return $out;
}
function base64_decode($string, $strict = false) {
    $alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    $bits = 0;
    $held = 0;
    $out = "";
    $at = 0;
    while ($at < strlen($string)) {
        $c = $string[$at];
        $at = $at + 1;
        if ($c === "=") { break; }
        $worth = strpos($alphabet, $c);
        if ($worth === false) { continue; }
        $held = ($held << 6) | $worth;
        $bits = $bits + 6;
        if ($bits >= 8) {
            $bits = $bits - 8;
            $out = $out . chr(($held >> $bits) & 255);
        }
    }
    return $out;
}

// Two pieces of text weighed byte by byte: below nought where the first
// comes first, above where it comes after, nought where they are the
// same. The letters may be taken as alike either way, and only so many
// of them looked at.
function strcmp($string1, $string2) { return __weighed_text((string) $string1, (string) $string2, null, false); }
function strcasecmp($string1, $string2) { return __weighed_text((string) $string1, (string) $string2, null, true); }
function strncmp($string1, $string2, $length) { return __weighed_text((string) $string1, (string) $string2, $length, false); }
function strncasecmp($string1, $string2, $length) { return __weighed_text((string) $string1, (string) $string2, $length, true); }
function __weighed_text($a, $b, $length, $fold) {
    if ($length !== null) { $a = substr($a, 0, $length); $b = substr($b, 0, $length); }
    if ($fold) { $a = strtolower($a); $b = strtolower($b); }
    $at = 0;
    while ($at < strlen($a) && $at < strlen($b)) {
        $one = ord($a[$at]);
        $two = ord($b[$at]);
        if ($one !== $two) { return $one < $two ? -1 : 1; }
        $at = $at + 1;
    }
    if (strlen($a) === strlen($b)) { return 0; }
    return strlen($a) < strlen($b) ? -1 : 1;
}
