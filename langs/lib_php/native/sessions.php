// What a run keeps for a visitor between one request and the next: a
// name for the visitor, and a file under that name holding what was put
// aside. Written in PHP because it is PHP's, not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

define("PHP_SESSION_DISABLED", 0);
define("PHP_SESSION_NONE", 1);
define("PHP_SESSION_ACTIVE", 2);

// What a program puts aside for the visitor. It stands empty until a
// session begins, as the reference has it stand.
$_SESSION = array();
$__session_id = "";
$__session_going = false;
$__session_named = null;
$__session_where = null;

// The run has no clock and nothing to draw a number out of the air with,
// so a visitor who comes with no name of their own is always given the
// same one. A visitor who brings one — the request carries it under the
// name the session goes by — keeps it, which is what tells two of them
// apart when there is anything to tell them apart by.
function __session_fresh_id() { return "b4d09f8e5a1c47e2af36d0b8c92e7f15"; }

function session_name($named = null) {
    global $__session_named;
    $was = $__session_named;
    if ($was === null) {
        $was = ini_get("session.name");
        if ($was === false || $was === "") { $was = "PHPSESSID"; }
    }
    if ($named !== null) { $__session_named = (string)$named; }
    return $was;
}
// Where the files are kept. Where the run was told of nowhere, they go
// where everything else without a home goes.
function session_save_path($where = null) {
    global $__session_where;
    $was = $__session_where;
    if ($was === null) {
        $was = ini_get("session.save_path");
        if ($was === false) { $was = ""; }
    }
    if ($where !== null) { $__session_where = (string)$where; }
    return $was;
}
function session_module_name($named = null) { return "files"; }
function session_status() {
    global $__session_going;
    return $__session_going ? PHP_SESSION_ACTIVE : PHP_SESSION_NONE;
}
// The name this visitor goes by. Setting one while a session is going is
// refused, since the session is already being kept under the old one.
function session_id($given = null) {
    global $__session_id, $__session_going;
    $was = $__session_id;
    if ($given !== null && !$__session_going) { $__session_id = (string)$given; }
    return $was;
}
// The file this session is kept in: the place they are kept, and the
// visitor's name with sess_ before it, which is what the reference calls
// its files.
function __session_file() {
    global $__session_id;
    $where = session_save_path();
    if ($where === "") { $where = sys_get_temp_dir(); }
    if (!ends_with($where, "/")) { $where = $where . "/"; }
    return $where . "sess_" . $__session_id;
}
// What is put aside, written out where it can be. Where the place they
// are kept is not there to write into, the session still goes on; it is
// only not kept, and there is nowhere for the run to say so.
function __session_keep() {
    global $_SESSION;
    return file_put_contents(__session_file(), __session_written($_SESSION)) !== false;
}
// A session begins under the name the request brought, or under one the
// run gives it. The file is made as it begins, whether anything is put
// aside or not, so that it stands there for the rest of the run.
function session_start($options = array()) {
    global $_SESSION, $__session_going, $__session_id;
    if ($__session_going) {
        __complaint_say(__complaint_word(E_NOTICE), "session_start(): Ignoring session_start() because a session is already active");
        return true;
    }
    if ($__session_id === "") {
        $brought = session_name();
        if (isset($_COOKIE[$brought]) && $_COOKIE[$brought] !== "") { $__session_id = (string)$_COOKIE[$brought]; }
        else { $__session_id = __session_fresh_id(); }
    }
    $__session_going = true;
    $_SESSION = array();
    $path = __session_file();
    if (file_exists($path)) {
        $held = __file_read($path);
        if ($held !== false) { $_SESSION = __session_taken($held); }
    }
    return __session_keep();
}
// What was put aside is written out and the session let go of, so that
// nothing more is kept under this name until one begins again.
function session_write_close() {
    global $__session_going;
    if (!$__session_going) { return false; }
    __session_keep();
    $__session_going = false;
    return true;
}
function session_commit() { return session_write_close(); }
// The session is let go of without what was put aside being written out.
function session_abort() {
    global $__session_going;
    if (!$__session_going) { return false; }
    $__session_going = false;
    return true;
}
function session_unset() {
    global $_SESSION, $__session_going;
    if (!$__session_going) { return false; }
    $_SESSION = array();
    return true;
}
// The file goes and the session with it. What a program still holds in
// hand is left alone, as the reference leaves it.
function session_destroy() {
    global $__session_going;
    if (!$__session_going) {
        __complaint_say(__complaint_word(E_WARNING), "session_destroy(): Trying to destroy uninitialized session");
        return false;
    }
    $path = __session_file();
    if (file_exists($path)) { unlink($path); }
    $__session_going = false;
    return true;
}

// A session file is a run of names, each with what it stands for written
// after it in the way the reference writes a value out: the kind of the
// value, then what it holds, and for text how much of it there is.
function __session_written($held) {
    $out = "";
    foreach ($held as $named => $value) {
        $out = $out . $named . "|" . __session_value_out($value);
    }
    return $out;
}
function __session_value_out($value) {
    if ($value === null) { return "N;"; }
    if (is_bool($value)) { return "b:" . ($value ? "1" : "0") . ";"; }
    if (is_int($value)) { return "i:" . $value . ";"; }
    if (is_float($value)) { return "d:" . strval($value) . ";"; }
    if (is_array($value)) {
        $out = "a:" . count($value) . ":{";
        foreach ($value as $named => $held) {
            $out = $out . __session_value_out($named) . __session_value_out($held);
        }
        return $out . "}";
    }
    $text = (string)$value;
    return "s:" . strlen($text) . ":\"" . $text . "\";";
}

// Reading one back again. The place the reading has got to is kept here
// rather than handed about, since a value written inside another is read
// by the same routine going round again.
$__session_at = 0;
function __session_taken($text) {
    global $__session_at;
    $out = array();
    $__session_at = 0;
    while ($__session_at < strlen($text)) {
        $upto = strpos($text, "|", $__session_at);
        if ($upto === false) { return $out; }
        $named = substr($text, $__session_at, $upto - $__session_at);
        $__session_at = $upto + 1;
        $out[$named] = __session_value_in($text);
    }
    return $out;
}
function __session_value_in($text) {
    global $__session_at;
    $kind = $text[$__session_at];
    if ($kind === "N") { $__session_at = $__session_at + 2; return null; }
    $__session_at = $__session_at + 2;
    if ($kind === "s") {
        $wide = __session_number_in($text, ":");
        $held = substr($text, $__session_at + 1, $wide);
        $__session_at = $__session_at + $wide + 3;
        return $held;
    }
    if ($kind === "a") {
        $many = __session_number_in($text, ":");
        $__session_at = $__session_at + 1;
        $out = array();
        $each = 0;
        while ($each < $many) {
            $named = __session_value_in($text);
            $out[$named] = __session_value_in($text);
            $each = $each + 1;
        }
        $__session_at = $__session_at + 1;
        return $out;
    }
    $said = __session_number_in($text, ";");
    if ($kind === "b") { return $said === "1"; }
    if ($kind === "d") { return real_of($said); }
    return whole_of($said);
}
// What stands between here and the next mark, the reading left just past
// the mark itself.
function __session_number_in($text, $mark) {
    global $__session_at;
    $upto = strpos($text, $mark, $__session_at);
    if ($upto === false) { $upto = strlen($text); }
    $said = substr($text, $__session_at, $upto - $__session_at);
    $__session_at = $upto + 1;
    return $said;
}

// A run told to begin a session before the program starts begins one
// here, which is the first thing the program finds already done.
$__session_auto = ini_get("session.auto_start");
if ($__session_auto !== false && $__session_auto !== "" && $__session_auto !== "0" && $__session_auto !== "off" && $__session_auto !== "Off") {
    session_start();
}
