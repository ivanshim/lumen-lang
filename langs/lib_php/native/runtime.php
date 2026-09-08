// PHP's own constants and a few of its library functions, written in PHP
// for the same reason its exception classes are: they are PHP's, not the
// kernel's. Hand-written; scripts/port_examples.py leaves native/ alone.

define("PHP_EOL", "\n");
define("PHP_INT_MAX", 9223372036854775807);
// Written as a subtraction because the literal 9223372036854775808 is
// one past the widest whole number, and so a real: php-src spells its
// own smallest whole number this way for the same reason.
define("PHP_INT_MIN", -9223372036854775807 - 1);
define("PHP_INT_SIZE", 8);
define("PHP_FLOAT_DIG", 15);
define("PHP_VERSION", "8.4.0");
define("PHP_MAJOR_VERSION", 8);
define("PHP_MINOR_VERSION", 4);
define("PHP_OS", "Linux");
define("PHP_OS_FAMILY", "Linux");
define("PHP_ZTS", 0);
define("PHP_DEBUG", 0);
define("DIRECTORY_SEPARATOR", "/");
define("M_PI", 3.14159265358979323846);
define("M_E", 2.71828182845904523536);
define("E_ERROR", 1);
define("E_WARNING", 2);
define("E_PARSE", 4);
define("E_NOTICE", 8);
define("E_CORE_ERROR", 16);
define("E_CORE_WARNING", 32);
define("E_COMPILE_ERROR", 64);
define("E_COMPILE_WARNING", 128);
define("E_USER_ERROR", 256);
define("E_USER_WARNING", 512);
define("E_USER_NOTICE", 1024);
define("E_STRICT", 2048);
define("E_RECOVERABLE_ERROR", 4096);
define("E_DEPRECATED", 8192);
define("E_USER_DEPRECATED", 16384);
define("E_ALL", 32767);
define("SORT_REGULAR", 0);
define("SORT_NUMERIC", 1);
define("SORT_STRING", 2);
define("COUNT_NORMAL", 0);
define("COUNT_RECURSIVE", 1);

// The settings a kernel run from a command line has nothing to change.
function error_reporting($level = null) { return E_ALL; }
// The settings a run was started with are carried in the environment,
// each under the name it has with PHP_INI_ before it, which is how the
// host tells the run about them. A setting written while the run goes
// is kept beside them and answered from there afterwards.
$__settings = array();
function ini_get($name) {
    global $__settings;
    if (array_key_exists($name, $__settings)) { return $__settings[$name]; }
    $carried = 'PHP_INI_' . $name;
    if (array_key_exists($carried, $_ENV)) { return $_ENV[$carried]; }
    return false;
}
// Only a setting the language has may be written to; a name that is
// none of its own is refused, whatever the run was started with. These
// are the ones this definition knows.
function ini_set($name, $value) {
    global $__settings;
    $known = array('precision', 'serialize_precision', 'memory_limit', 'max_execution_time', 'error_reporting', 'display_errors', 'log_errors', 'default_charset', 'internal_encoding', 'input_encoding', 'output_encoding', 'include_path', 'date.timezone', 'output_buffering', 'zend.assertions', 'assert.exception');
    if (!in_array($name, $known)) { return false; }
    $was = ini_get($name);
    $__settings[$name] = (string)$value;
    return $was;
}
// ini_alter is ini_set under its older name; ini_restore drops what was
// written while the run went, so that the setting the run was started
// with answers again.
function ini_alter($name, $value) { return ini_set($name, $value); }
function ini_restore($name) {
    global $__settings;
    unset($__settings[$name]);
    return null;
}

// The whole number a piece of text opens with, as a setting's size is
// read: a sign, then figures counted in sixteens after 0x, in twos
// after 0b, in eights after a lone nought, and in tens otherwise.
// Anything that is not a figure ends the number.
function leading_whole($text) {
    $sign = 1;
    $at = 0;
    if (strlen($text) > 0 && ($text[0] === "-" || $text[0] === "+")) {
        if ($text[0] === "-") { $sign = -1; }
        $at = 1;
    }
    $rest = strtolower(substr($text, $at));
    $base = 10;
    if (starts_with($rest, "0x")) { $base = 16; $rest = substr($rest, 2); }
    elseif (starts_with($rest, "0b")) { $base = 2; $rest = substr($rest, 2); }
    elseif (strlen($rest) > 1 && $rest[0] === "0") { $base = 8; $rest = substr($rest, 1); }
    $figures = substr(base_digits(), 0, $base);
    $end = 0;
    while ($end < strlen($rest) && strpos($figures, $rest[$end]) !== false) { $end = $end + 1; }
    if ($end == 0) { return 0; }
    return $sign * base_to_number(substr($rest, 0, $end), $base);
}

// A setting's size: the number the text opens with, times what the last
// letter of the whole stands for. A thousand and twenty-four for k, that
// many times over again for m, and once more for g; any other letter
// stands for nothing, and the number is read without it.
function ini_parse_quantity($text) {
    $text = trim($text);
    if ($text === "") { return 0; }
    $last = strtolower($text[strlen($text) - 1]);
    $times = 1;
    if ($last === "k") { $times = 1024; }
    if ($last === "m") { $times = 1048576; }
    if ($last === "g") { $times = 1073741824; }
    if ($times > 1) { $text = substr($text, 0, strlen($text) - 1); }
    return leading_whole($text) * $times;
}

function set_error_handler($handler, $levels = 32767) { return null; }
function restore_error_handler() { return true; }
function set_exception_handler($handler) { return null; }
function error_log($message) { return true; }
function extension_loaded($name) { return false; }
function function_exists($name) { return false; }
function gc_collect_cycles() { return 0; }
function memory_get_usage($real = false) { return 0; }

// A key is taken as the array takes one, so 7 and "7" name one place.
function array_key_exists($key, $array) {
    foreach ($array as $k => $v) {
        if ($k == $key) { return true; }
    }
    return false;
}

function in_array($needle, $haystack, $strict = false) {
    foreach ($haystack as $v) {
        if ($v == $needle) { return true; }
    }
    return false;
}

function array_keys($array) {
    $out = array();
    foreach ($array as $k => $v) { $out[] = $k; }
    return $out;
}

function array_values($array) {
    $out = array();
    foreach ($array as $v) { $out[] = $v; }
    return $out;
}

function array_merge($first, $second) {
    $out = array();
    foreach ($first as $v) { $out[] = $v; }
    foreach ($second as $v) { $out[] = $v; }
    return $out;
}

function array_flip($array) {
    $out = array();
    foreach ($array as $k => $v) { $out[$v] = $k; }
    return $out;
}

function array_reverse($array) {
    $out = array();
    $n = count($array);
    $i = $n - 1;
    while ($i >= 0) { $out[] = $array[$i]; $i = $i - 1; }
    return $out;
}

function array_pop(&$array) {
    $n = count($array);
    if ($n == 0) { return null; }
    $last = $array[$n - 1];
    $kept = array();
    $i = 0;
    while ($i < $n - 1) { $kept[] = $array[$i]; $i = $i + 1; }
    $array = $kept;
    return $last;
}

function array_shift(&$array) {
    $n = count($array);
    if ($n == 0) { return null; }
    $first = $array[0];
    $kept = array();
    $i = 1;
    while ($i < $n) { $kept[] = $array[$i]; $i = $i + 1; }
    $array = $kept;
    return $first;
}

function array_unshift(&$array, $value) {
    $kept = array($value);
    foreach ($array as $v) { $kept[] = $v; }
    $array = $kept;
    return count($kept);
}

function implode($glue, $pieces) {
    $out = "";
    $first = true;
    foreach ($pieces as $piece) {
        if ($first) { $out = $out . $piece; $first = false; }
        else { $out = $out . $glue . $piece; }
    }
    return $out;
}

function join($glue, $pieces) { return implode($glue, $pieces); }

// Text as a web address carries it: a letter, a figure and a few marks
// stand as they are, and everything else is written as a percent and the
// two figures of its code. The older spelling writes a space as a plus
// and does not spare the tilde.
function url_encoded($text, $plus_for_space, $tilde_spared) {
    $out = "";
    $at = 0;
    while ($at < strlen($text)) {
        $c = $text[$at];
        $n = ord($c);
        $plain = ($n >= 97 && $n <= 122) || ($n >= 65 && $n <= 90) || ($n >= 48 && $n <= 57);
        $plain = $plain || $n == 45 || $n == 46 || $n == 95 || ($tilde_spared && $n == 126);
        if ($plain) {
            $out = $out . $c;
        } elseif ($n == 32 && $plus_for_space) {
            $out = $out . "+";
        } else {
            $out = $out . "%" . strtoupper(str_pad(dechex($n), 2, "0", 0));
        }
        $at = $at + 1;
    }
    return $out;
}
function urlencode($text) { return url_encoded($text, true, false); }
function rawurlencode($text) { return url_encoded($text, false, true); }

function str_repeat($text, $times) {
    $out = "";
    $i = 0;
    while ($i < $times) { $out = $out . $text; $i = $i + 1; }
    return $out;
}

// gettype answers with the name of a kind, which is what PHP has always
// given back, so these read that name.
function is_int($value) { return gettype($value) === "integer"; }
function is_integer($value) { return is_int($value); }
function is_long($value) { return is_int($value); }
function is_string($value) { return gettype($value) === "string"; }
function is_bool($value) { return gettype($value) === "boolean"; }
function is_array($value) { return gettype($value) === "array"; }
function is_null($value) { return $value === null; }
function is_float($value) { return gettype($value) === "double"; }
function is_double($value) { return is_float($value); }
function is_numeric($value) {
    if (is_int($value) || is_float($value)) { return true; }
    if (!is_string($value)) { return false; }
    $text = trim($value);
    $at = 0;
    if (starts_with($text, "+") || starts_with($text, "-")) { $at = 1; }
    $digits = 0;
    $points = 0;
    while ($at < strlen($text)) {
        $letter = $text[$at];
        if (is_digit($letter)) { $digits = $digits + 1; }
        elseif ($letter === ".") { $points = $points + 1; }
        elseif (($letter === "e" || $letter === "E") && $digits > 0) {
            $rest = substr($text, $at + 1);
            if (starts_with($rest, "+") || starts_with($rest, "-")) { $rest = substr($rest, 1); }
            if (strlen($rest) == 0) { return false; }
            $each = 0;
            while ($each < strlen($rest)) {
                if (!is_digit($rest[$each])) { return false; }
                $each = $each + 1;
            }
            return $digits > 0 && $points < 2;
        }
        else { return false; }
        $at = $at + 1;
    }
    return $digits > 0 && $points < 2;
}

// A value as a number, whether it was written as one or spelled out.
function whole_of($value) {
    if (is_int($value)) { return $value; }
    if (is_bool($value)) { if ($value) { return 1; } return 0; }
    if ($value === null) { return 0; }
    if (is_string($value)) {
        if (!is_numeric($value)) { return 0; }
        return intval(0 + trim($value));
    }
    return intval($value);
}

function real_of($value) {
    if (is_int($value) || is_float($value)) { return floatval($value); }
    if (is_bool($value)) { if ($value) { return 1.0; } return 0.0; }
    if ($value === null) { return 0.0; }
    if (is_string($value) && is_numeric($value)) { return floatval(0 + trim($value)); }
    return 0.0;
}
function is_object($value) { return false; }
function is_callable($value) { return false; }

// What a run from a command line has nothing to answer with, and the
// few library functions that only need what is already here.
function sys_get_temp_dir() { return "/tmp"; }
function header($line, $replace = true, $code = 0) { return null; }
function headers_sent() { return false; }
function headers_list() { return array(); }
// What the run writes out may be kept aside and let go again. The
// kernel holds the text; the handlers a program hands over are kept
// here, one for each keeping, and run over the text as it is let go.
$__handlers = array();
function ob_start($handler = null) {
    global $__handlers;
    __output_hold();
    $__handlers[] = $handler;
    return true;
}
function ob_get_contents() { return __output_held(); }
function ob_get_level() { return __output_depth(); }
function __output_handler() {
    global $__handlers;
    if (count($__handlers) == 0) { return null; }
    $last = $__handlers[count($__handlers) - 1];
    array_pop($__handlers);
    return $last;
}
function ob_end_clean() {
    if (__output_depth() == 0) { return false; }
    __output_handler();
    return __output_drop();
}
function ob_get_clean() {
    if (__output_depth() == 0) { return false; }
    $held = __output_held();
    __output_handler();
    __output_drop();
    return $held;
}
function ob_end_flush() {
    if (__output_depth() == 0) { return false; }
    $held = __output_held();
    $handler = __output_handler();
    __output_drop();
    if ($handler !== null) { $held = $handler($held, 8); }
    echo $held;
    return true;
}
function ob_get_flush() {
    if (__output_depth() == 0) { return false; }
    $held = __output_held();
    ob_end_flush();
    return $held;
}
function ob_flush() {
    if (__output_depth() == 0) { return false; }
    $held = __output_held();
    __output_drop();
    echo $held;
    __output_hold();
    return true;
}
function ob_clean() {
    if (__output_depth() == 0) { return false; }
    __output_drop();
    __output_hold();
    return true;
}
function flush() { return null; }
function usleep($micro) { return null; }
function sleep($seconds) { return 0; }

function getenv($name = null) {
    if ($name === null) { return $_ENV; }
    if (array_key_exists($name, $_ENV)) { return $_ENV[$name]; }
    return false;
}

// A file sent with a request is written where the program can read it,
// under a name this host gives it.
function is_uploaded_file($path) {
    return is_string($path) && has_substring($path, "lumenup");
}

function move_uploaded_file($from, $to) { return is_uploaded_file($from); }

function strtoupper($text) { return string_to_upper($text); }
function strtolower($text) { return string_to_lower($text); }
function str_contains($haystack, $needle) { return has_substring($haystack, $needle); }
function str_starts_with($haystack, $needle) { return starts_with($haystack, $needle); }
function str_ends_with($haystack, $needle) { return ends_with($haystack, $needle); }
function strrev($text) { return reverse_characters($text); }
function ucfirst($text) { return capitalize_first_word($text); }
function ucwords($text) { return capitalize_words($text); }
function ltrim($text) { return trim_start($text); }
function rtrim($text) { return trim_end($text); }
function abs($n) { if ($n < 0) { return 0 - $n; } return $n; }
function max($a, $b) { if ($a > $b) { return $a; } return $b; }
function min($a, $b) { if ($a < $b) { return $a; } return $b; }
function intdiv($a, $b) { return intval($a / $b); }

// Text taken apart and put back together, the way PHP's own library
// spells it.  What is written here is PHP, so the kernel need learn
// nothing of it.

function substr($text, $start, $length = null) {
    $size = strlen($text);
    if ($start < 0) {
        $start = $size + $start;
        if ($start < 0) { $start = 0; }
    }
    if ($start > $size) { return ""; }
    if ($length === null) {
        $stop = $size;
    } elseif ($length < 0) {
        $stop = $size + $length;
    } else {
        $stop = $start + $length;
    }
    if ($stop > $size) { $stop = $size; }
    if ($stop < $start) { return ""; }
    return substring($text, $start, $stop);
}

function strpos($haystack, $needle, $offset = 0) {
    $at = index_of(substr($haystack, $offset), $needle);
    if ($at < 0) { return false; }
    return $at + $offset;
}

function strstr($haystack, $needle, $before = false) {
    $at = index_of($haystack, $needle);
    if ($at < 0) { return false; }
    if ($before) { return substr($haystack, 0, $at); }
    return substr($haystack, $at);
}

function str_replace($search, $replace, $subject) {
    $out = "";
    $rest = $subject;
    $width = strlen($search);
    if ($width == 0) { return $subject; }
    while (true) {
        $at = index_of($rest, $search);
        if ($at < 0) { return $out . $rest; }
        $out = $out . substr($rest, 0, $at) . $replace;
        $rest = substr($rest, $at + $width);
    }
}

function str_pad($text, $width, $pad = " ", $side = 1) {
    $short = $width - strlen($text);
    if ($short <= 0 || strlen($pad) == 0) { return $text; }
    $filler = "";
    while (strlen($filler) < $short) { $filler = $filler . $pad; }
    $filler = substr($filler, 0, $short);
    if ($side == 0) { return $filler . $text; }
    if ($side == 2) {
        $left = intdiv($short, 2);
        return substr($filler, 0, $left) . $text . substr($filler, 0, $short - $left);
    }
    return $text . $filler;
}

function str_split($text, $width = 1) {
    $pieces = array();
    $at = 0;
    $size = strlen($text);
    if ($size == 0) { return array(""); }
    while ($at < $size) {
        array_push($pieces, substr($text, $at, $width));
        $at = $at + $width;
    }
    return $pieces;
}

function explode($apart, $text) {
    $pieces = array();
    $rest = $text;
    $width = strlen($apart);
    if ($width == 0) { return false; }
    while (true) {
        $at = index_of($rest, $apart);
        if ($at < 0) {
            array_push($pieces, $rest);
            return $pieces;
        }
        array_push($pieces, substr($rest, 0, $at));
        $rest = substr($rest, $at + $width);
    }
}

// A number written in another base, and read back from one.

function base_digits() { return "0123456789abcdefghijklmnopqrstuvwxyz"; }

function number_in_base($n, $base) {
    if ($n == 0) { return "0"; }
    $digits = base_digits();
    $out = "";
    $left = $n;
    while ($left > 0) {
        $out = $digits[$left % $base] . $out;
        $left = intdiv($left, $base);
    }
    return $out;
}

function base_to_number($text, $base) {
    $digits = base_digits();
    $total = 0;
    $at = 0;
    while ($at < strlen($text)) {
        $place = index_of($digits, char_to_lower($text[$at]));
        if ($place >= 0 && $place < $base) { $total = $total * $base + $place; }
        $at = $at + 1;
    }
    return $total;
}

function dechex($n) { return number_in_base($n, 16); }
function hexdec($text) { return base_to_number($text, 16); }
function decbin($n) { return number_in_base($n, 2); }
function bindec($text) { return base_to_number($text, 2); }
function decoct($n) { return number_in_base($n, 8); }
function octdec($text) { return base_to_number($text, 8); }

function bin2hex($text) {
    $out = "";
    $at = 0;
    while ($at < strlen($text)) {
        $out = $out . str_pad(dechex(ord($text[$at])), 2, "0", 0);
        $at = $at + 1;
    }
    return $out;
}

function hex2bin($text) {
    $out = "";
    $at = 0;
    while ($at + 1 < strlen($text)) {
        $out = $out . chr(hexdec(substr($text, $at, 2)));
        $at = $at + 2;
    }
    return $out;
}

// The rest of what a program expects to find already there.

function phpversion($extension = null) { return PHP_VERSION; }
function zend_version() { return "4.0.0"; }
function php_sapi_name() { return "cli"; }
function php_uname($mode = "a") { return PHP_OS; }
function setlocale($category, $locale) { return false; }
function date_default_timezone_set($zone) { return true; }
function date_default_timezone_get() { return "UTC"; }
function register_shutdown_function($work) { return null; }
function trigger_error($message, $level = 1024) { return true; }
function realpath($path) { return $path; }
function basename($path) {
    $at = strlen($path) - 1;
    while ($at >= 0) {
        if ($path[$at] == "/") { return substr($path, $at + 1); }
        $at = $at - 1;
    }
    return $path;
}
function dirname($path) {
    $at = strlen($path) - 1;
    while ($at > 0) {
        if ($path[$at] == "/") { return substr($path, 0, $at); }
        $at = $at - 1;
    }
    return ".";
}

define("LC_ALL", 6);
define("LC_COLLATE", 3);
define("LC_CTYPE", 0);
define("LC_MONETARY", 4);
define("LC_NUMERIC", 1);
define("LC_TIME", 2);
define("LC_MESSAGES", 5);
define("STR_PAD_RIGHT", 1);
define("STR_PAD_LEFT", 0);
define("STR_PAD_BOTH", 2);

// A value written the way a program would write it, which is what
// var_export means by exporting one.

function var_export_string($value, $indent) {
    $kind = gettype($value);
    if ($kind === "NULL") { return "NULL"; }
    if ($kind === "boolean") { if ($value) { return "true"; } return "false"; }
    if ($kind === "string") { return "'" . str_replace("'", "\\'", str_replace("\\", "\\\\", $value)) . "'"; }
    if ($kind === "array") {
        $pad = str_repeat(" ", $indent);
        $out = "array (\n";
        foreach ($value as $key => $held) {
            $out = $out . $pad . "  " . var_export_string($key, $indent + 2) . " => ";
            if (gettype($held) === "array") { $out = $out . "\n" . $pad . "  "; }
            $out = $out . var_export_string($held, $indent + 2) . ",\n";
        }
        return $out . $pad . ")";
    }
    if ($kind === "double") {
        $shown = strval($value);
        if (!str_contains($shown, ".") && !str_contains($shown, "E") && !str_contains($shown, "e")) {
            return $shown . ".0";
        }
        return $shown;
    }
    return strval($value);
}

function var_export($value, $give_back = false) {
    $out = var_export_string($value, 0);
    if ($give_back) { return $out; }
    print($out);
    return null;
}

// Text laid out to a pattern, as printf has always spelled it.

function pad_to($text, $width, $filler, $to_the_left) {
    if (strlen($text) >= $width) { return $text; }
    $room = str_repeat($filler, $width - strlen($text));
    if ($to_the_left) { return $text . $room; }
    return $room . $text;
}

function rounded_string($number, $places) {
    $below = $number < 0;
    if ($below) { $number = 0 - $number; }
    $scale = 10 ** $places;
    $whole = intval(($number * $scale) + 0.5);
    $shown = strval($whole);
    if ($places > 0) {
        $shown = pad_to($shown, $places + 1, "0", false);
        $shown = substr($shown, 0, strlen($shown) - $places) . "." . substr($shown, strlen($shown) - $places);
    }
    if ($below) { return "-" . $shown; }
    return $shown;
}

function one_conversion($letter, $value, $places) {
    if ($letter === "d" || $letter === "i") { return strval(whole_of($value)); }
    if ($letter === "u") { $n = whole_of($value); if ($n < 0) { $n = $n + 18446744073709551616; } return strval($n); }
    if ($letter === "s") { if ($places === null) { return strval($value); } return substr(strval($value), 0, $places); }
    if ($letter === "f" || $letter === "F") { if ($places === null) { $places = 6; } return rounded_string(real_of($value), $places); }
    if ($letter === "x") { return dechex(whole_of($value)); }
    if ($letter === "X") { return strtoupper(dechex(whole_of($value))); }
    if ($letter === "o") { return decoct(whole_of($value)); }
    if ($letter === "b") { return decbin(whole_of($value)); }
    if ($letter === "c") { return chr(whole_of($value)); }
    return strval($value);
}

function sprintf_over($pattern, $values) {
    $out = "";
    $at = 0;
    $taken = 0;
    $size = strlen($pattern);
    while ($at < $size) {
        if ($pattern[$at] !== "%") {
            $out = $out . $pattern[$at];
            $at = $at + 1;
            continue;
        }
        $at = $at + 1;
        if ($at < $size && $pattern[$at] === "%") { $out = $out . "%"; $at = $at + 1; continue; }
        $to_the_left = false;
        $filler = " ";
        $signed = false;
        while ($at < $size) {
            $flag = $pattern[$at];
            if ($flag === "-") { $to_the_left = true; }
            elseif ($flag === "0") { $filler = "0"; }
            elseif ($flag === "+") { $signed = true; }
            elseif ($flag === " ") { $filler = " "; }
            elseif ($flag === "'") { $at = $at + 1; $filler = $pattern[$at]; }
            else { break; }
            $at = $at + 1;
        }
        $width = 0;
        while ($at < $size && is_digit($pattern[$at])) {
            $width = $width * 10 + character_to_value($pattern[$at]);
            $at = $at + 1;
        }
        $places = null;
        if ($at < $size && $pattern[$at] === ".") {
            $at = $at + 1;
            $places = 0;
            while ($at < $size && is_digit($pattern[$at])) {
                $places = $places * 10 + character_to_value($pattern[$at]);
                $at = $at + 1;
            }
        }
        if ($at >= $size) { return $out; }
        $letter = $pattern[$at];
        $at = $at + 1;
        $value = null;
        if ($taken < count($values)) { $value = $values[$taken]; }
        $taken = $taken + 1;
        $shown = one_conversion($letter, $value, $places);
        if ($signed && $letter !== "s" && !starts_with($shown, "-")) { $shown = "+" . $shown; }
        $out = $out . pad_to($shown, $width, $filler, $to_the_left);
    }
    return $out;
}

function sprintf($pattern) {
    $values = func_get_args();
    array_shift($values);
    return sprintf_over($pattern, $values);
}

function printf($pattern) {
    $values = func_get_args();
    array_shift($values);
    $out = sprintf_over($pattern, $values);
    print($out);
    return strlen($out);
}

function vsprintf($pattern, $values) { return sprintf_over($pattern, $values); }
function vprintf($pattern, $values) { $out = sprintf_over($pattern, $values); print($out); return strlen($out); }

function number_format($number, $places = 0, $point = ".", $between = ",") {
    $shown = rounded_string($number, $places);
    $sign = "";
    if (starts_with($shown, "-")) { $sign = "-"; $shown = substr($shown, 1); }
    $whole = $shown;
    $rest = "";
    $dot = index_of($shown, ".");
    if ($dot >= 0) { $whole = substr($shown, 0, $dot); $rest = substr($shown, $dot + 1); }
    $grouped = "";
    $left = strlen($whole);
    while ($left > 3) {
        $grouped = $between . substr($whole, $left - 3, 3) . $grouped;
        $left = $left - 3;
    }
    $grouped = substr($whole, 0, $left) . $grouped;
    if ($places > 0) { return $sign . $grouped . $point . $rest; }
    return $sign . $grouped;
}
