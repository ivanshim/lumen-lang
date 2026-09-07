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
