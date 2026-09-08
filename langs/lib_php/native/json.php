// Values written as JSON and read back from it. Written in PHP because
// it is PHP's and not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

define("JSON_HEX_TAG", 1);
define("JSON_HEX_AMP", 2);
define("JSON_HEX_APOS", 4);
define("JSON_HEX_QUOT", 8);
define("JSON_FORCE_OBJECT", 16);
define("JSON_NUMERIC_CHECK", 32);
define("JSON_UNESCAPED_SLASHES", 64);
define("JSON_PRETTY_PRINT", 128);
define("JSON_UNESCAPED_UNICODE", 256);
define("JSON_PARTIAL_OUTPUT_ON_ERROR", 512);
define("JSON_PRESERVE_ZERO_FRACTION", 1024);
define("JSON_INVALID_UTF8_IGNORE", 1048576);
define("JSON_INVALID_UTF8_SUBSTITUTE", 2097152);
define("JSON_THROW_ON_ERROR", 4194304);
define("JSON_OBJECT_AS_ARRAY", 1);
define("JSON_BIGINT_AS_STRING", 2);
define("JSON_ERROR_NONE", 0);
define("JSON_ERROR_SYNTAX", 4);

// A value written as JSON. An array whose keys are nought, one, two and
// so on to the last is written as a list; any other array, and any
// thing, is written as an object.
function json_encode($value, $flags = 0, $depth = 512) {
    __json_said(JSON_ERROR_NONE);
    return __json_written($value, $flags, 0);
}
function __json_written($value, $flags, $deep) {
    if ($value === null) { return "null"; }
    if (is_bool($value)) { return $value ? "true" : "false"; }
    if (is_int($value)) { return strval($value); }
    if (is_float($value)) {
        if ($value == (float) (int) $value && $value < 9223372036854775808.0 && $value > -9223372036854775808.0) {
            $whole = strval((int) $value);
            return $flags & JSON_PRESERVE_ZERO_FRACTION ? $whole . ".0" : $whole;
        }
        return strval($value);
    }
    if (is_string($value)) { return __json_quoted($value, $flags); }
    if (is_object($value)) { return __json_pairs(get_object_vars($value), $flags, $deep); }
    if (!is_array($value)) { return "null"; }
    $a_list = !($flags & JSON_FORCE_OBJECT);
    $want = 0;
    foreach ($value as $k => $v) {
        if ($k !== $want) { $a_list = false; }
        $want = $want + 1;
    }
    if (!$a_list) { return __json_pairs($value, $flags, $deep); }
    $pieces = array();
    foreach ($value as $v) { $pieces[] = __json_written($v, $flags, $deep + 1); }
    return __json_laid_out("[", $pieces, "]", $flags, $deep);
}
function __json_pairs($array, $flags, $deep) {
    $pieces = array();
    foreach ($array as $k => $v) {
        $said = __json_quoted((string) $k, $flags) . ($flags & JSON_PRETTY_PRINT ? ": " : ":");
        $pieces[] = $said . __json_written($v, $flags, $deep + 1);
    }
    return __json_laid_out("{", $pieces, "}", $flags, $deep);
}
function __json_laid_out($open, $pieces, $close, $flags, $deep) {
    if (!($flags & JSON_PRETTY_PRINT)) { return $open . implode(",", $pieces) . $close; }
    if (count($pieces) === 0) { return $open . $close; }
    $in = str_repeat("    ", $deep + 1);
    $back = str_repeat("    ", $deep);
    return $open . "\n" . $in . implode(",\n" . $in, $pieces) . "\n" . $back . $close;
}

// Text written as JSON writes it: the quote, the backslash and the
// characters below a space stand for themselves by name.
function __json_quoted($text, $flags) {
    $out = "\"";
    $at = 0;
    while ($at < strlen($text)) {
        $c = $text[$at];
        $code = ord($c);
        if ($c === "\"") { $out = $out . "\\\""; }
        else if ($c === "\\") { $out = $out . "\\\\"; }
        else if ($c === "/") { $out = $out . ($flags & JSON_UNESCAPED_SLASHES ? "/" : "\\/"); }
        else if ($c === "\n") { $out = $out . "\\n"; }
        else if ($c === "\r") { $out = $out . "\\r"; }
        else if ($c === "\t") { $out = $out . "\\t"; }
        else if ($code === 8) { $out = $out . "\\b"; }
        else if ($code === 12) { $out = $out . "\\f"; }
        else if ($code < 32) { $out = $out . "\\u" . __json_four($code); }
        else { $out = $out . $c; }
        $at = $at + 1;
    }
    return $out . "\"";
}
function __json_four($code) {
    $digits = "0123456789abcdef";
    return $digits[($code >> 12) & 15] . $digits[($code >> 8) & 15] . $digits[($code >> 4) & 15] . $digits[$code & 15];
}

// JSON read back. Objects come back as things unless the caller asks
// for arrays. Text that is no JSON at all answers with nothing, and
// says so through json_last_error.
function json_decode($json, $associative = null, $depth = 512, $flags = 0) {
    __json_said(JSON_ERROR_NONE);
    $as_arrays = $associative === true || ($flags & JSON_OBJECT_AS_ARRAY);
    $at = 0;
    $held = __json_read((string) $json, $at, $as_arrays);
    if ($held === null && __json_said(null) !== JSON_ERROR_NONE) { return null; }
    $at = __json_past_blanks((string) $json, $at);
    if ($at < strlen((string) $json)) {
        __json_said(JSON_ERROR_SYNTAX);
        return null;
    }
    return $held;
}
function json_last_error() { return __json_said(null); }
function json_last_error_msg() {
    return __json_said(null) === JSON_ERROR_NONE ? "No error" : "Syntax error";
}
// What went amiss with the last reading, kept between calls.
function __json_said($set) {
    static $held = 0;
    if ($set !== null) { $held = $set; }
    return $held;
}
function __json_past_blanks($text, $at) {
    while ($at < strlen($text)) {
        $c = $text[$at];
        if ($c !== " " && $c !== "\t" && $c !== "\n" && $c !== "\r") { break; }
        $at = $at + 1;
    }
    return $at;
}
function __json_read($text, &$at, $as_arrays) {
    $at = __json_past_blanks($text, $at);
    if ($at >= strlen($text)) { __json_said(JSON_ERROR_SYNTAX); return null; }
    $c = $text[$at];
    if ($c === "{") { return __json_read_object($text, $at, $as_arrays); }
    if ($c === "[") { return __json_read_list($text, $at, $as_arrays); }
    if ($c === "\"") { return __json_read_text($text, $at); }
    if (substr($text, $at, 4) === "true") { $at = $at + 4; return true; }
    if (substr($text, $at, 5) === "false") { $at = $at + 5; return false; }
    if (substr($text, $at, 4) === "null") { $at = $at + 4; return null; }
    return __json_read_number($text, $at);
}
function __json_read_object($text, &$at, $as_arrays) {
    $at = $at + 1;
    $held = array();
    $at = __json_past_blanks($text, $at);
    if ($at < strlen($text) && $text[$at] === "}") { $at = $at + 1; return $as_arrays ? $held : __json_thing($held); }
    while (true) {
        $at = __json_past_blanks($text, $at);
        if ($at >= strlen($text) || $text[$at] !== "\"") { __json_said(JSON_ERROR_SYNTAX); return null; }
        $key = __json_read_text($text, $at);
        $at = __json_past_blanks($text, $at);
        if ($at >= strlen($text) || $text[$at] !== ":") { __json_said(JSON_ERROR_SYNTAX); return null; }
        $at = $at + 1;
        $held[$key] = __json_read($text, $at, $as_arrays);
        if (__json_said(null) !== JSON_ERROR_NONE) { return null; }
        $at = __json_past_blanks($text, $at);
        if ($at < strlen($text) && $text[$at] === ",") { $at = $at + 1; continue; }
        if ($at < strlen($text) && $text[$at] === "}") { $at = $at + 1; break; }
        __json_said(JSON_ERROR_SYNTAX);
        return null;
    }
    return $as_arrays ? $held : __json_thing($held);
}
// An array of pairs as a thing of no class of its own, which is what
// JSON's objects come back as.
function __json_thing($pairs) {
    $thing = new stdClass;
    foreach ($pairs as $k => $v) {
        $named = (string) $k;
        $thing->$named = $v;
    }
    return $thing;
}
function __json_read_list($text, &$at, $as_arrays) {
    $at = $at + 1;
    $held = array();
    $at = __json_past_blanks($text, $at);
    if ($at < strlen($text) && $text[$at] === "]") { $at = $at + 1; return $held; }
    while (true) {
        $held[] = __json_read($text, $at, $as_arrays);
        if (__json_said(null) !== JSON_ERROR_NONE) { return null; }
        $at = __json_past_blanks($text, $at);
        if ($at < strlen($text) && $text[$at] === ",") { $at = $at + 1; continue; }
        if ($at < strlen($text) && $text[$at] === "]") { $at = $at + 1; break; }
        __json_said(JSON_ERROR_SYNTAX);
        return null;
    }
    return $held;
}
function __json_read_text($text, &$at) {
    $at = $at + 1;
    $out = "";
    while ($at < strlen($text)) {
        $c = $text[$at];
        if ($c === "\"") { $at = $at + 1; return $out; }
        if ($c === "\\") {
            $at = $at + 1;
            if ($at >= strlen($text)) { break; }
            $next = $text[$at];
            if ($next === "n") { $out = $out . "\n"; }
            else if ($next === "t") { $out = $out . "\t"; }
            else if ($next === "r") { $out = $out . "\r"; }
            else if ($next === "b") { $out = $out . chr(8); }
            else if ($next === "f") { $out = $out . chr(12); }
            else if ($next === "u") {
                $code = __json_code_of(substr($text, $at + 1, 4));
                $at = $at + 4;
                $out = $out . __json_utf8($code);
            }
            else { $out = $out . $next; }
            $at = $at + 1;
            continue;
        }
        $out = $out . $c;
        $at = $at + 1;
    }
    __json_said(JSON_ERROR_SYNTAX);
    return null;
}
function __json_code_of($four) {
    $held = 0;
    $at = 0;
    while ($at < strlen($four)) {
        $held = $held * 16 + hexdec($four[$at]);
        $at = $at + 1;
    }
    return $held;
}
function __json_utf8($code) {
    if ($code < 128) { return chr($code); }
    if ($code < 2048) { return chr(192 + ($code >> 6)) . chr(128 + ($code & 63)); }
    return chr(224 + ($code >> 12)) . chr(128 + (($code >> 6) & 63)) . chr(128 + ($code & 63));
}
function __json_read_number($text, &$at) {
    $from = $at;
    if ($at < strlen($text) && $text[$at] === "-") { $at = $at + 1; }
    $a_real = false;
    while ($at < strlen($text)) {
        $c = $text[$at];
        if ($c >= "0" && $c <= "9") { $at = $at + 1; continue; }
        if ($c === "." || $c === "e" || $c === "E" || $c === "+" || $c === "-") { $a_real = true; $at = $at + 1; continue; }
        break;
    }
    $written = substr($text, $from, $at - $from);
    if ($written === "" || $written === "-") { __json_said(JSON_ERROR_SYNTAX); return null; }
    return $a_real ? (float) $written : (int) $written;
}
