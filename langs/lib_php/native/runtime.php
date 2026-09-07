// PHP's own constants and a few of its library functions, written in PHP
// for the same reason its exception classes are: they are PHP's, not the
// kernel's. Hand-written; scripts/port_examples.py leaves native/ alone.

define("PHP_EOL", "\n");
define("PHP_INT_MAX", 9223372036854775807);
define("PHP_INT_MIN", -9223372036854775808);
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
function ini_set($name, $value) { return false; }
function ini_get($name) { return false; }
function set_error_handler($handler, $levels = 32767) { return null; }
function restore_error_handler() { return true; }
function set_exception_handler($handler) { return null; }
function error_log($message) { return true; }
function extension_loaded($name) { return false; }
function function_exists($name) { return false; }
function gc_collect_cycles() { return 0; }
function memory_get_usage($real = false) { return 0; }

function array_key_exists($key, $array) {
    foreach ($array as $k => $v) {
        if ($k === $key) { return true; }
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

function str_repeat($text, $times) {
    $out = "";
    $i = 0;
    while ($i < $times) { $out = $out . $text; $i = $i + 1; }
    return $out;
}

// gettype answers with a kind, and the definition names the kinds under
// system.kind.*, so these read the names it gives rather than strings.
function is_int($value) { return gettype($value) === integer; }
function is_integer($value) { return is_int($value); }
function is_long($value) { return is_int($value); }
function is_string($value) { return gettype($value) === string; }
function is_bool($value) { return gettype($value) === boolean; }
function is_array($value) { return gettype($value) === array; }
function is_null($value) { return gettype($value) === NULL; }
function is_float($value) { return gettype($value) === double; }
function is_double($value) { return is_float($value); }
function is_numeric($value) { return is_int($value) || is_float($value); }
function is_object($value) { return false; }
function is_callable($value) { return false; }

// What a run from a command line has nothing to answer with, and the
// few library functions that only need what is already here.
function set_time_limit($seconds) { return true; }
function sys_get_temp_dir() { return "/tmp"; }
function header($line, $replace = true, $code = 0) { return null; }
function headers_sent() { return false; }
function headers_list() { return array(); }
function ob_start($handler = null) { return true; }
function ob_get_clean() { return ""; }
function ob_end_clean() { return true; }
function ob_get_level() { return 0; }
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
function php_sapi_name() { return "cli"; }
function php_uname($mode = "a") { return PHP_OS; }
function setlocale($category, $locale) { return false; }
function date_default_timezone_set($zone) { return true; }
function date_default_timezone_get() { return "UTC"; }
function register_shutdown_function($work) { return null; }
function trigger_error($message, $level = 1024) { return true; }
function file_exists($path) { return false; }
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
