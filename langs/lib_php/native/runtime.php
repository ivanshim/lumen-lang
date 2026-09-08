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
define("PHP_BUILD_DATE", "Sep  8 2026 00:00:00");
define("INF", 1.0e400);
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
// Every kind of complaint a program may ask to be told of. The kind
// once called strict is no longer one of them, though its number is
// still spelled.
define("E_ALL", 30719);
define("SORT_REGULAR", 0);
define("SORT_NUMERIC", 1);
define("SORT_STRING", 2);
define("COUNT_NORMAL", 0);
define("COUNT_RECURSIVE", 1);
define("PHP_OUTPUT_HANDLER_START", 1);
define("PHP_OUTPUT_HANDLER_WRITE", 0);
define("PHP_OUTPUT_HANDLER_CLEAN", 2);
define("PHP_OUTPUT_HANDLER_FLUSH", 4);
define("PHP_OUTPUT_HANDLER_FINAL", 8);
define("PHP_OUTPUT_HANDLER_END", 8);
define("PHP_OUTPUT_HANDLER_CONT", 0);

// The settings a kernel run from a command line has nothing to change.
// Which kinds of complaint are to be said at all, and the routine a
// program has put in the way of them. A run starts saying all of them
// and with no routine of its own in the way.
// What the host found amiss in the request before the program ran is
// said first of all, as PHP says it.
// A setting on its way out is said to be, before the program runs.
function __say_settings_outworn() {
    global $__started_with;
    if (!is_array($__started_with)) { return null; }
    foreach ($__started_with as $name => $said) {
        if ($name === "report_memleaks") {
            echo "\nDeprecated: PHP Startup: Directive '" . $name . "' is deprecated in Unknown on line 0\n";
        }
    }
    return null;
}
function __say_request_amiss() {
    global $__request_amiss;
    if (!is_array($__request_amiss)) { return null; }
    foreach ($__request_amiss as $told) {
        $said = $told[0];
        if (count($told) == 3) { $said = sprintf($told[0], $told[1], $told[2]); }
        elseif (count($told) == 2) { $said = sprintf($told[0], $told[1]); }
        echo "\nWarning: " . $said . " in Unknown on line 0\n";
    }
    return null;
}
$__reporting = __ini_reporting();
$__error_handler = null;
__hook_as_needed();
__say_request_amiss();
__say_settings_outworn();
function error_reporting($level = null) {
    global $__reporting;
    $was = $__reporting;
    if ($level !== null) {
        $__reporting = whole_of($level);
        __hook_as_needed();
    }
    return $was;
}
// The run's own complaints come this way only where the program wants
// something done with them: a routine of its own in their way, or some
// kinds not to be said at all.
function __hook_as_needed() {
    global $__error_handler, $__reporting;
    if ($__error_handler !== null || $__reporting != E_ALL) {
        __complaint_handler('__from_the_run');
    } else {
        __complaint_handler(null);
    }
}
// The settings a run was started with are carried in the environment,
// each under the name it has with PHP_INI_ before it, which is how the
// host tells the run about them. A setting written while the run goes
// is kept beside them and answered from there afterwards.
$__settings = array();
__room_at_start();
// What a setting stands at where the run was started with nothing said
// about it. A name not among these has no value at all until one is set.
function __ini_default($name) {
    if ($name === "default_charset") { return "UTF-8"; }
    if ($name === "input_encoding") { return ""; }
    if ($name === "internal_encoding") { return ""; }
    if ($name === "output_encoding") { return ""; }
    return false;
}
function ini_get($name) {
    global $__settings;
    if (array_key_exists($name, $__settings)) { return $__settings[$name]; }
    global $__started_with;
    if (array_key_exists($name, $__started_with)) {
        $held = $__started_with[$name];
        // A setting counted in kinds of complaint is written as an
        // expression over their words, and answers as the number.
        if ($name === "error_reporting") { return (string)__ini_number($held); }
        return $held;
    }
    return __ini_default($name);
}
// Only a setting the language has may be written to; a name that is
// none of its own is refused, whatever the run was started with. These
// are the ones this definition knows.
function ini_set($name, $value) {
    global $__settings;
    $known = array('precision', 'serialize_precision', 'memory_limit', 'max_execution_time', 'error_reporting', 'display_errors', 'log_errors', 'default_charset', 'internal_encoding', 'input_encoding', 'output_encoding', 'include_path', 'date.timezone', 'output_buffering', 'zend.assertions', 'assert.exception');
    if (!in_array($name, $known)) { return false; }
    $was = ini_get($name);
    if ($name === 'memory_limit') {
        $held = __room_allowed((string)$value);
        if ($held[1]) { __complaint_say(__complaint_word(E_WARNING), __room_said((string)$value, $held[0])); }
        $__settings[$name] = $held[0];
        return $was;
    }
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
$__whole_used = 0;
function leading_whole($text) {
    global $__whole_used;
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
    $__whole_used = strlen($text) - strlen($rest) + $end;
    if ($end == 0) { return 0; }
    return $sign * base_to_number(substr($rest, 0, $end), $base);
}

// A setting's size: the number the text opens with, times what the last
// letter of the whole stands for. A thousand and twenty-four for k, that
// many times over again for m, and once more for g; any other letter
// stands for nothing, and the number is read without it.
function ini_parse_quantity($text) {
    global $__whole_used;
    $whole = trim($text);
    if ($whole === "") { return 0; }
    $last = strtolower($whole[strlen($whole) - 1]);
    $times = 1;
    if ($last === "k") { $times = 1024; }
    if ($last === "m") { $times = 1048576; }
    if ($last === "g") { $times = 1073741824; }
    $body = ($times > 1) ? substr($whole, 0, strlen($whole) - 1) : $whole;
    $found = leading_whole($body);
    $over = trim(substr($body, $__whole_used));
    // Anything past the number that is not the letter standing for a
    // multiplier is read past, and said to be read past, since a run
    // that once took such a setting still takes it.
    if ($times > 1) {
        if ($over !== "") {
            __complaint_say(__complaint_word(E_WARNING), 'Invalid quantity "' . $text . '", interpreting as "' . $found . ' ' . $last . '" for backwards compatibility');
        }
    } elseif ($over !== "") {
        $mark = ord($last);
        if ($mark >= 97 && $mark <= 122) {
            __complaint_say(__complaint_word(E_WARNING), 'Invalid quantity "' . $text . '": unknown multiplier "' . $last . '", interpreting as "' . $found . '" for backwards compatibility');
        } else {
            __complaint_say(__complaint_word(E_WARNING), 'Invalid quantity "' . $text . '", interpreting as "' . $found . '" for backwards compatibility');
        }
    }
    return $found * $times;
}

// How much room a run may take is held down to the most it may be given.
// A wish for more than that is turned down and said to be; a wish for no
// limit at all is quietly brought down to it.
function __room_allowed($wanted) {
    $most = ini_get('max_memory_limit');
    if ($most === false || $most === "") { return array($wanted, false); }
    $ceiling = ini_parse_quantity($most);
    if ($ceiling <= 0) { return array($wanted, false); }
    $asked = ini_parse_quantity($wanted);
    if ($asked < 0) { return array($most, false); }
    if ($asked > $ceiling) { return array($most, true); }
    return array($wanted, false);
}
function __room_said($wanted, $most) {
    return "Failed to set memory_limit to " . ini_parse_quantity($wanted) . " bytes. Setting to max_memory_limit instead (currently: " . ini_parse_quantity($most) . " bytes)";
}
// The room the run was started with, brought down where it must be.
function __room_at_start() {
    global $__settings, $__started_with;
    if (!array_key_exists('memory_limit', $__started_with)) { return null; }
    $wanted = $__started_with['memory_limit'];
    $held = __room_allowed($wanted);
    if ($held[1]) {
        echo "\nWarning: " . __room_said($wanted, $held[0]) . " in Unknown on line 0\n";
    }
    $__settings['memory_limit'] = $held[0];
    return null;
}
function set_error_handler($handler, $levels = 30719) {
    global $__error_handler;
    $was = $__error_handler;
    $__error_handler = $handler;
    __hook_as_needed();
    return $was;
}
function restore_error_handler() {
    global $__error_handler;
    $__error_handler = null;
    __hook_as_needed();
    return true;
}
// The number a program knows a kind of complaint by, from the word the
// run writes it under.
function __complaint_number($word) {
    if ($word === "Warning") { return E_WARNING; }
    if ($word === "Deprecated") { return E_DEPRECATED; }
    if ($word === "Fatal error") { return E_ERROR; }
    return E_NOTICE;
}
// A complaint the run itself made. The program's routine takes it; where
// there is none, or where its kind is one being said, answering false
// leaves the run to write it out in its own words.
function __from_the_run($word, $message, $file, $line) {
    global $__error_handler, $__reporting;
    $level = __complaint_number($word);
    if ($__error_handler !== null) {
        $handler = $__error_handler;
        $handler($level, $message, $file, $line);
        return true;
    }
    if (($__reporting & $level) == 0) { return true; }
    return false;
}
// The word each kind of complaint is written under, and the number a
// program knows it by.
function __complaint_word($level) {
    if ($level == E_USER_ERROR || $level == E_ERROR) { return "Fatal error"; }
    if ($level == E_USER_WARNING || $level == E_WARNING) { return "Warning"; }
    if ($level == E_USER_DEPRECATED || $level == E_DEPRECATED) { return "Deprecated"; }
    return "Notice";
}
// A routine put in the way of a value nobody took: the run hands it
// over rather than telling it in its own words.
$__exception_handler = null;
function set_exception_handler($handler) {
    global $__exception_handler;
    $was = $__exception_handler;
    $__exception_handler = $handler;
    __uncaught_handler($handler);
    return $was;
}
function restore_exception_handler() {
    global $__exception_handler;
    $__exception_handler = null;
    __uncaught_handler(null);
    return true;
}
// A message put where the run keeps them. Sending one on as mail is not
// something a run of this kind does, so saying to send one with nowhere
// to send it to is turned down.
function error_log($message, $sort = 0, $where = null, $headers = null) {
    if ($sort == 1) { return $where !== null; }
    return true;
}
function extension_loaded($name) { return false; }
// The classes this run has bound, and the routines. Everything this PHP
// has of its own is written in PHP, so there are no functions from
// outside the language to list beside them.
function get_declared_classes() { return __classes_bound(); }
// The class a thing's class stands on, or that a class named stands on;
// false where it stands on none, as the reference answers.
function get_parent_class($of = null) {
    $under = __class_beneath($of);
    return $under === null ? false : $under;
}
function get_class($of) { return $of::class; }
function get_defined_functions($exclude_disabled = true) {
    if (func_num_args() > 0) {
        __complaint_say(__complaint_word(E_DEPRECATED), 'get_defined_functions(): The $exclude_disabled parameter has no effect since PHP 8.0');
    }
    return array("internal" => __words_spelled(), "user" => __routines_bound());
}
// Whether a routine of that name is there to be called: one the
// language spells of its own, or one the program or this library has
// written. A name is told apart however it is written, as a call is.
function function_exists($name) {
    $wanted = strtolower($name);
    foreach (__words_spelled() as $word) {
        if (strtolower($word) === $wanted) { return true; }
    }
    foreach (__routines_bound() as $bound) {
        if (strtolower($bound) === $wanted) { return true; }
    }
    return false;
}
function gc_collect_cycles() { return 0; }
function memory_get_usage($real = false) { return 0; }

// A key is taken as the array takes one, so 7 and "7" name one place.
// Nothing standing where a key should is still read as the empty piece
// of text, which is on its way out and said to be.
function array_key_exists($key, $array) {
    if ($key === null) {
        __complaint_say(__complaint_word(E_DEPRECATED), "Using null as the key parameter for array_key_exists() is deprecated, use an empty string instead");
    }
    foreach ($array as $k => $v) {
        if ($k == $key) { return true; }
    }
    return false;
}

// Every place of an array handed to a routine, with its key beside it
// and whatever else was given after that. The value is handed over as a
// cell, so a routine taking one writes the array in place.
function array_walk(&$array, $what, $given = null) {
    $handed = func_num_args() > 2;
    foreach ($array as $key => &$value) {
        if ($handed) { $what($value, $key, $given); } else { $what($value, $key); }
    }
    return true;
}
function in_array($needle, $haystack, $strict = false) {
    foreach ($haystack as $v) {
        if ($v == $needle) { return true; }
    }
    return false;
}
// Where the value stands, or false where it stands nowhere. Text and a
// number are alike where the strict form is not asked for, as they are
// for `in_array`.
function array_search($needle, $haystack, $strict = false) {
    foreach ($haystack as $k => $v) {
        if ($strict) {
            if ($v === $needle) { return $k; }
        } elseif ($v == $needle) {
            return $k;
        }
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

// Arrays laid one after another. A key that is text keeps its place and
// the last value written under it stands; a key that is a number is
// counted afresh, so nothing is ever written over by its number.
function array_merge() {
    $out = array();
    foreach (func_get_args() as $array) {
        foreach ($array as $k => $v) {
            if (is_string($k)) { $out[$k] = $v; } else { $out[] = $v; }
        }
    }
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

// The last place goes and the rest stay as they are, cells and all: a
// name tied to one of them is tied to it still.
function array_pop(&$array) {
    $n = count($array);
    if ($n == 0) { return null; }
    $last = $array[$n - 1];
    unset($array[$n - 1]);
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
// The place an array is looked at from. Nothing here moves that place —
// a walk leaves it where it stood, and this language has no word for
// moving it — so it is always the first place, and these two say what
// stands there. The words for moving it are left out rather than
// written to move nothing, since a word that lies is worse than none.
// Both take the array as a value and not as a cell: what they answer is
// read out of it and nothing is written back, and asking them of
// something that is not a binding is nothing to complain of.
function current($array) {
    if (!is_array($array)) { return false; }
    foreach ($array as $value) { return $value; }
    return false;
}
function key($array) {
    if (!is_array($array)) { return null; }
    foreach ($array as $at => $value) { return $at; }
    return null;
}
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
function is_object($value) { return gettype($value) === "object"; }
function is_callable($value) { return false; }

// What a run from a command line has nothing to answer with, and the
// few library functions that only need what is already here.
function sys_get_temp_dir() {
    $named = ini_get("sys_temp_dir");
    if ($named !== false && $named !== "") { return $named; }
    return "/tmp";
}
function header($line, $replace = true, $code = 0) { return null; }
// Headers go out with the first thing written, so anything written at
// all means they are gone.
function headers_sent() { return __output_begun(); }
function headers_list() { return array(); }
// A routine to run as the headers go out. Where they are already gone
// there is nothing left to run it for; where they are not, they go out
// when the run ends and nothing has sent them before.
$__header_callback = null;
$__headers_gone = false;
function header_register_callback($callback) {
    global $__header_callback;
    if (__output_begun()) { return false; }
    $__header_callback = $callback;
    __at_end('__headers_going');
    return true;
}
function __headers_going() {
    global $__header_callback, $__headers_gone;
    if ($__headers_gone) { return true; }
    $__headers_gone = true;
    $callback = $__header_callback;
    if ($callback !== null) { $callback(); }
    return true;
}
// The arguments the run was started with are part of what the run knows
// about itself, under the host's names for them as well as their own.
// A run reached over the web knows them only where it is told to; one
// started from the command line always does. Where the run was not to
// be told about itself at all, nothing is put here either.
$__over_the_web = isset($_SERVER['REQUEST_METHOD']);
$__told_of_args = ini_get('register_argc_argv');
$__groups = ini_get('variables_order');
if ($__groups === false) { $__groups = 'EGPCS'; }
$__knows_itself = strpos($__groups, 'S') !== false;
$__told_on = $__told_of_args !== false && $__told_of_args !== '0' && $__told_of_args !== 'off';
if ($__knows_itself && !$__over_the_web) {
    $_SERVER['argv'] = $argv;
    $_SERVER['argc'] = $argc;
} elseif ($__knows_itself && $__told_on) {
    // A run reached over the web has no arguments of its own, so where
    // it is told to know some it takes the words of the query, which the
    // reference says is a thing to be leaving behind.
    __complaint_say(__complaint_word(E_DEPRECATED), "Deriving \$_SERVER['argv'] from the query string is deprecated. Configure register_argc_argv=0 to turn this message off");
    $__query = isset($_SERVER['QUERY_STRING']) ? $_SERVER['QUERY_STRING'] : '';
    $_SERVER['argv'] = $__query === '' ? array() : explode('+', $__query);
    $_SERVER['argc'] = count($_SERVER['argv']);
}
// What the run writes out may be kept aside and let go again. The
// kernel holds the text; the handlers a program hands over are kept
// here, one for each keeping, and run over the text as it is let go.
$__handlers = array();
$__flushing = false;
$__started = array();
function ob_start($handler = null) {
    global $__handlers, $__started, $__flushing;
    // What is still being kept when the run ends is let go then. The
    // kernel lets go of what it holds by itself; a handler is the one
    // part it cannot run, so only a keeping given one is let go by hand.
    if ($handler !== null && !$__flushing) { $__flushing = true; __at_end('__let_go_all'); }
    __output_hold();
    $__handlers[] = $handler;
    $__started[] = false;
    return true;
}
function __let_go_all() {
    while (__output_depth() > 0) { ob_end_flush(); }
}
function ob_get_contents() { return __output_held(); }
function ob_get_level() { return __output_depth(); }
// What the innermost keeping holds, run through the handler the program
// gave for it. The handler is told whether this is the first it has seen
// of this keeping and whether it is the last.
function __run_handler($held, $final, $why = 0) {
    global $__handlers, $__started;
    $at = count($__handlers) - 1;
    if ($at < 0) { return $held; }
    $handler = $__handlers[$at];
    if ($handler === null) { return $held; }
    $mode = $why;
    if ($why == 0) { $mode = $final ? PHP_OUTPUT_HANDLER_FINAL : PHP_OUTPUT_HANDLER_FLUSH; }
    if (!$__started[$at]) { $mode = $mode | PHP_OUTPUT_HANDLER_START; }
    $__started[$at] = true;
    return $handler($held, $mode);
}
function __forget_handler() {
    global $__handlers, $__started;
    array_pop($__handlers);
    array_pop($__started);
}
// Letting a keeping go without writing it out. The handler is told the
// keeping is being emptied and let go for good, as the reference tells
// it, and what it answers with is thrown away with the rest.
function ob_end_clean() {
    if (__output_depth() == 0) { return false; }
    __run_handler(__output_held(), true, PHP_OUTPUT_HANDLER_CLEAN | PHP_OUTPUT_HANDLER_FINAL);
    __forget_handler();
    return __output_drop();
}
function ob_get_clean() {
    if (__output_depth() == 0) { return false; }
    $held = __output_held();
    __run_handler($held, true, PHP_OUTPUT_HANDLER_CLEAN | PHP_OUTPUT_HANDLER_FINAL);
    __forget_handler();
    __output_drop();
    return $held;
}
function ob_end_flush() {
    if (__output_depth() == 0) { return false; }
    $held = __run_handler(__output_held(), true);
    __forget_handler();
    __output_drop();
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
    $held = __run_handler(__output_held(), false);
    __output_drop();
    echo $held;
    __output_hold();
    return true;
}
function ob_clean() {
    if (__output_depth() == 0) { return false; }
    // The handler is told the keeping was emptied, and what it answers
    // with is thrown away with the rest; it has still seen this keeping.
    __run_handler(__output_held(), false, PHP_OUTPUT_HANDLER_CLEAN);
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
function abs($n) { if ($n < 0) { return 0 - $n; } return $n; }
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
// The clock and the calendar. The kernel is asked only how far the clock
// has come since the start of 1970; turning that into a date, and a date
// back into it, is arithmetic and belongs here. Everything is reckoned
// in UTC, which is the one zone this run keeps.
function time() { return __clock(); }
function microtime($as_float = false) {
    $now = __clock();
    if ($as_float) { return $now + 0.0; }
    return "0.00000000 " . $now;
}
function hrtime($as_number = false) {
    $now = __clock();
    if ($as_number) { return $now * 1000000000; }
    return array($now, 0);
}
// Dividing where what is left over is never below nought, so that dates
// before 1970 count back the way dates after it count on.
function __floor_div($a, $b) {
    $whole = intdiv($a, $b);
    if ($a % $b != 0 && ($a < 0) != ($b < 0)) { $whole = $whole - 1; }
    return $whole;
}
function __floor_rem($a, $b) { return $a - __floor_div($a, $b) * $b; }
// The day a date stands on, counting from the first of January 1970.
// Howard Hinnant's reckoning: the year is turned about so that a leap
// day falls at the end of it, and the four-hundred-year turn of the
// calendar is counted out whole.
function __days_of_date($year, $month, $day) {
    $y = $month <= 2 ? $year - 1 : $year;
    $era = __floor_div($y, 400);
    $of_era = $y - $era * 400;
    $of_year = intdiv(153 * ($month + ($month > 2 ? -3 : 9)) + 2, 5) + $day - 1;
    $of_era_days = $of_era * 365 + intdiv($of_era, 4) - intdiv($of_era, 100) + $of_year;
    return $era * 146097 + $of_era_days - 719468;
}
// The date a day stands on, the same reckoning read backwards.
function __date_of_days($days) {
    $days = $days + 719468;
    $era = __floor_div($days, 146097);
    $of_era = $days - $era * 146097;
    $of_era_year = intdiv($of_era - intdiv($of_era, 1460) + intdiv($of_era, 36524) - intdiv($of_era, 146096), 365);
    $year = $of_era_year + $era * 400;
    $of_year = $of_era - (365 * $of_era_year + intdiv($of_era_year, 4) - intdiv($of_era_year, 100));
    $mp = intdiv(5 * $of_year + 2, 153);
    $day = $of_year - intdiv(153 * $mp + 2, 5) + 1;
    $month = $mp + ($mp < 10 ? 3 : -9);
    if ($month <= 2) { $year = $year + 1; }
    return array($year, $month, $day);
}
function __is_leap_year($year) {
    if ($year % 4 != 0) { return false; }
    if ($year % 100 != 0) { return true; }
    return $year % 400 == 0;
}
function __days_in_month($month, $year) {
    $lengths = array(31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31);
    if ($month == 2 && __is_leap_year($year)) { return 29; }
    return $lengths[$month - 1];
}
// Everything a moment is made of, which the readers below share.
function __moment($when) {
    $days = __floor_div($when, 86400);
    $of_day = __floor_rem($when, 86400);
    $ymd = __date_of_days($days);
    $year = $ymd[0];
    $month = $ymd[1];
    $day = $ymd[2];
    return array(
        "seconds" => __floor_rem($of_day, 60),
        "minutes" => __floor_rem(intdiv($of_day, 60), 60),
        "hours" => intdiv($of_day, 3600),
        "mday" => $day,
        "wday" => __floor_rem($days + 4, 7),
        "mon" => $month,
        "year" => $year,
        "yday" => $days - __days_of_date($year, 1, 1),
        "weekday" => __weekday_name(__floor_rem($days + 4, 7)),
        "month" => __month_name($month),
        0 => $when,
    );
}
function __weekday_name($wday) {
    $names = array("Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday");
    return $names[$wday];
}
function __month_name($month) {
    $names = array("January", "February", "March", "April", "May", "June",
                   "July", "August", "September", "October", "November", "December");
    return $names[$month - 1];
}
function getdate($when = null) {
    if ($when === null) { $when = __clock(); }
    return __moment($when);
}
// A date written out as the clock counts it. A part left out is taken
// from the moment the clock stands at now, as the reference takes it.
function mktime($hour = null, $minute = null, $second = null, $month = null, $day = null, $year = null) {
    $now = __moment(__clock());
    if ($hour === null) { $hour = $now["hours"]; }
    if ($minute === null) { $minute = $now["minutes"]; }
    if ($second === null) { $second = $now["seconds"]; }
    if ($month === null) { $month = $now["mon"]; }
    if ($day === null) { $day = $now["mday"]; }
    if ($year === null) { $year = $now["year"]; }
    // A month or a day past its own bounds counts on into the next, as
    // the reference lets it.
    $year = $year + __floor_div($month - 1, 12);
    $month = __floor_rem($month - 1, 12) + 1;
    return __days_of_date($year, $month, $day) * 86400 + $hour * 3600 + $minute * 60 + $second;
}
function gmmktime($hour = null, $minute = null, $second = null, $month = null, $day = null, $year = null) {
    return mktime($hour, $minute, $second, $month, $day, $year);
}
function checkdate($month, $day, $year) {
    if ($month < 1 || $month > 12 || $day < 1 || $year < 1 || $year > 32767) { return false; }
    return $day <= __days_in_month($month, $year);
}
// A number written out with noughts before it so that it fills the width.
function __padded($n, $width) {
    $out = (string) $n;
    while (strlen($out) < $width) { $out = "0" . $out; }
    return $out;
}
function __ordinal_suffix($day) {
    if ($day % 100 >= 11 && $day % 100 <= 13) { return "th"; }
    $last = $day % 10;
    if ($last == 1) { return "st"; }
    if ($last == 2) { return "nd"; }
    if ($last == 3) { return "rd"; }
    return "th";
}
// A moment written out letter by letter, as the reference writes it. A
// letter with a backslash before it stands for itself.
function date($pattern, $when = null) {
    if ($when === null) { $when = __clock(); }
    $m = __moment($when);
    $out = "";
    $at = 0;
    $reach = strlen($pattern);
    while ($at < $reach) {
        $c = $pattern[$at];
        if ($c === "\\") {
            $at = $at + 1;
            if ($at < $reach) { $out = $out . $pattern[$at]; }
            $at = $at + 1;
            continue;
        }
        $out = $out . __date_letter($c, $m, $when);
        $at = $at + 1;
    }
    return $out;
}
function gmdate($pattern, $when = null) { return date($pattern, $when); }
function __date_letter($c, $m, $when) {
    if ($c === "d") { return __padded($m["mday"], 2); }
    if ($c === "j") { return (string) $m["mday"]; }
    if ($c === "S") { return __ordinal_suffix($m["mday"]); }
    if ($c === "D") { return substr($m["weekday"], 0, 3); }
    if ($c === "l") { return $m["weekday"]; }
    if ($c === "N") { return (string) ($m["wday"] == 0 ? 7 : $m["wday"]); }
    if ($c === "w") { return (string) $m["wday"]; }
    if ($c === "z") { return (string) $m["yday"]; }
    if ($c === "m") { return __padded($m["mon"], 2); }
    if ($c === "n") { return (string) $m["mon"]; }
    if ($c === "M") { return substr($m["month"], 0, 3); }
    if ($c === "F") { return $m["month"]; }
    if ($c === "t") { return (string) __days_in_month($m["mon"], $m["year"]); }
    if ($c === "L") { return __is_leap_year($m["year"]) ? "1" : "0"; }
    if ($c === "Y") { return (string) $m["year"]; }
    if ($c === "y") { return __padded($m["year"] % 100, 2); }
    if ($c === "H") { return __padded($m["hours"], 2); }
    if ($c === "G") { return (string) $m["hours"]; }
    if ($c === "h") { return __padded(__twelve_hour($m["hours"]), 2); }
    if ($c === "g") { return (string) __twelve_hour($m["hours"]); }
    if ($c === "i") { return __padded($m["minutes"], 2); }
    if ($c === "s") { return __padded($m["seconds"], 2); }
    if ($c === "a") { return $m["hours"] < 12 ? "am" : "pm"; }
    if ($c === "A") { return $m["hours"] < 12 ? "AM" : "PM"; }
    if ($c === "U") { return (string) $when; }
    if ($c === "e" || $c === "T") { return "UTC"; }
    if ($c === "P") { return "+00:00"; }
    if ($c === "O") { return "+0000"; }
    if ($c === "Z") { return "0"; }
    if ($c === "u") { return "000000"; }
    if ($c === "v") { return "000"; }
    return $c;
}
function __twelve_hour($hours) {
    $twelve = $hours % 12;
    return $twelve == 0 ? 12 : $twelve;
}

function register_shutdown_function($work, $a = null, $b = null, $c = null) {
    if ($c !== null) { return __at_end($work, $a, $b, $c); }
    if ($b !== null) { return __at_end($work, $a, $b); }
    if ($a !== null) { return __at_end($work, $a); }
    return __at_end($work);
}
// A complaint the program itself makes, which names its own kind: the
// kinds a program raises are told apart from the kinds a run raises,
// and only the number says which, so the routine in their way is handed
// the number and not the word the run would use.
function trigger_error($message, $level = 1024) {
    global $__error_handler, $__reporting;
    if ($__error_handler !== null) {
        $handler = $__error_handler;
        $handler($level, $message, __FILE__, __LINE__);
        return true;
    }
    if (($__reporting & $level) == 0) { return true; }
    return __complaint_say(__complaint_word($level), $message);
}
function user_error($message, $level = 1024) { return trigger_error($message, $level); }
// What a file holds. The body of the request the run was started with
// is a file a program may name, and reads the same however often it is
// read, since it is held as it came rather than drawn from.
function file_get_contents($path) {
    global $__request_body;
    if ($path === "php://input") {
        if (!is_string($__request_body)) { return ""; }
        return $__request_body;
    }
    return __file_read($path);
}
function realpath($path) { return $path; }
// Moving a file: what it held is written where it is going and taken
// from where it was. A file that is not there to move is said so and
// answered with false, as every other reading of one that is not there
// is answered.
function rename($from, $to) {
    $held = __file_read($from);
    if ($held === false) {
        __complaint_say(__complaint_word(E_WARNING), "rename(" . $from . "," . $to . "): No such file or directory");
        return false;
    }
    file_put_contents($to, $held);
    unlink($from);
    return true;
}
// Whether a name has been given a value that stands everywhere, and
// what that value is. Both are asked by working the name out, since a
// name that stands for nothing cannot be worked out at all.
function defined($name) {
    try {
        eval("return " . $name . ";");
    } catch (Error $e) {
        return false;
    }
    return true;
}
function constant($name) {
    return eval("return " . $name . ";");
}
// A claim a program makes about itself. Where the run is set to let
// them go by, it is not looked at; otherwise a claim that does not hold
// is raised, under the words the program gave for it where it gave any.
function assert($claim, $told = null) {
    if (ini_get("zend.assertions") === "-1") { return true; }
    if ($claim) { return true; }
    if ($told !== null && !is_string($told)) { throw $told; }
    if ($told !== null) { throw new AssertionError($told); }
    throw new AssertionError("assert(false)");
}
// One call of a trace written out the way PHP writes it: where the call
// stands, what it named, and enough of each argument to know it by.
function __frame_told($frame) {
    $named = isset($frame['class']) ? $frame['class'] . '::' . $frame['function'] : $frame['function'];
    $pieces = array();
    foreach ($frame['args'] as $given) {
        $pieces[] = __argument_told($given);
    }
    // A call made from inside the language itself stands nowhere the
    // program was written, and is written down as standing nowhere.
    $stood = isset($frame['file']) ? $frame['file'] . '(' . $frame['line'] . ')' : '[internal function]';
    return $stood . ': ' . $named . '(' . implode(', ', $pieces) . ')';
}
function __argument_told($given) {
    if (is_string($given)) {
        $kept = substr($given, 0, 15);
        return "'" . $kept . (strlen($given) > 15 ? "...'" : "'");
    }
    if (is_bool($given)) { return $given ? 'true' : 'false'; }
    if ($given === null) { return 'NULL'; }
    if (is_array($given)) { return 'Array'; }
    if (is_object($given)) { return 'Object(' . $given::class . ')'; }
    return (string) $given;
}
// The calls under way where the asking itself is left out, since a
// program asking for them is not one of the calls it wants to hear of.
function debug_backtrace() {
    $under = __calls();
    array_shift($under);
    return $under;
}
function debug_print_backtrace() {
    $under = __calls();
    array_shift($under);
    $at = 0;
    foreach ($under as $frame) {
        echo '#' . $at . ' ' . __frame_told($frame) . "\n";
        $at = $at + 1;
    }
}
// Calling what a value names, with whatever else was handed over. The
// value may be the name of a routine or a thing paired with the name of
// one of its methods; either stands where a routine stands.
function call_user_func($what) {
    $given = func_get_args();
    $count = func_num_args();
    if ($count <= 1) { return $what(); }
    if ($count == 2) { return $what($given[1]); }
    if ($count == 3) { return $what($given[1], $given[2]); }
    if ($count == 4) { return $what($given[1], $given[2], $given[3]); }
    if ($count == 5) { return $what($given[1], $given[2], $given[3], $given[4]); }
    return $what($given[1], $given[2], $given[3], $given[4], $given[5]);
}
function call_user_func_array($what, $given) {
    $count = count($given);
    if ($count == 0) { return $what(); }
    if ($count == 1) { return $what($given[0]); }
    if ($count == 2) { return $what($given[0], $given[1]); }
    if ($count == 3) { return $what($given[0], $given[1], $given[2]); }
    if ($count == 4) { return $what($given[0], $given[1], $given[2], $given[3]); }
    return $what($given[0], $given[1], $given[2], $given[3], $given[4]);
}
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

// A setting written as an expression over the words a language names its
// kinds of complaint by: the words and numbers taken together with the
// marks that work on bits. What binds tightest is a mark before a value,
// then both bits, then one bit, then either bit.
$__ini_at = 0;
function __ini_word($text) {
    global $__ini_at;
    while ($__ini_at < strlen($text) && $text[$__ini_at] === " ") { $__ini_at = $__ini_at + 1; }
    if ($__ini_at >= strlen($text)) { return ""; }
    $c = $text[$__ini_at];
    if (strpos("&|^~!()", $c) !== false) { $__ini_at = $__ini_at + 1; return $c; }
    $from = $__ini_at;
    while ($__ini_at < strlen($text) && strpos(" &|^~!()", $text[$__ini_at]) === false) { $__ini_at = $__ini_at + 1; }
    return substr($text, $from, $__ini_at - $from);
}
function __ini_peek($text) {
    global $__ini_at;
    $was = $__ini_at;
    $word = __ini_word($text);
    $__ini_at = $was;
    return $word;
}
function __ini_value($text) {
    $word = __ini_word($text);
    if ($word === "~") { return ~__ini_value($text); }
    if ($word === "!") { return __ini_value($text) ? 0 : 1; }
    if ($word === "(") { $held = __ini_either($text); __ini_word($text); return $held; }
    if ($word === "") { return 0; }
    return __ini_named($word);
}
// The words a setting may be written with, and what each is worth.
function __ini_named($word) {
    if ($word === "E_ERROR") { return E_ERROR; }
    if ($word === "E_WARNING") { return E_WARNING; }
    if ($word === "E_PARSE") { return E_PARSE; }
    if ($word === "E_NOTICE") { return E_NOTICE; }
    if ($word === "E_CORE_ERROR") { return E_CORE_ERROR; }
    if ($word === "E_CORE_WARNING") { return E_CORE_WARNING; }
    if ($word === "E_COMPILE_ERROR") { return E_COMPILE_ERROR; }
    if ($word === "E_COMPILE_WARNING") { return E_COMPILE_WARNING; }
    if ($word === "E_USER_ERROR") { return E_USER_ERROR; }
    if ($word === "E_USER_WARNING") { return E_USER_WARNING; }
    if ($word === "E_USER_NOTICE") { return E_USER_NOTICE; }
    if ($word === "E_STRICT") { return E_STRICT; }
    if ($word === "E_RECOVERABLE_ERROR") { return E_RECOVERABLE_ERROR; }
    if ($word === "E_DEPRECATED") { return E_DEPRECATED; }
    if ($word === "E_USER_DEPRECATED") { return E_USER_DEPRECATED; }
    if ($word === "E_ALL") { return E_ALL; }
    if ($word === "On" || $word === "on" || $word === "true" || $word === "yes") { return 1; }
    if ($word === "Off" || $word === "off" || $word === "false" || $word === "no" || $word === "none") { return 0; }
    return whole_of($word);
}
function __ini_both($text) {
    $held = __ini_value($text);
    while (__ini_peek($text) === "&") { __ini_word($text); $held = $held & __ini_value($text); }
    return $held;
}
function __ini_one($text) {
    $held = __ini_both($text);
    while (__ini_peek($text) === "^") { __ini_word($text); $held = $held ^ __ini_both($text); }
    return $held;
}
function __ini_either($text) {
    $held = __ini_one($text);
    while (__ini_peek($text) === "|") { __ini_word($text); $held = $held | __ini_one($text); }
    return $held;
}
// Which kinds of complaint the run was started with, where it was
// started with a setting for them at all.
function __ini_reporting() {
    global $__started_with;
    if (!array_key_exists('error_reporting', $__started_with)) { return E_ALL; }
    return __ini_number($__started_with['error_reporting']);
}
function __ini_number($text) {
    global $__ini_at;
    $__ini_at = 0;
    return __ini_either($text);
}
