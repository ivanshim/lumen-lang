// What the run makes of the files it writes: the mask it holds them
// down by, the rights each was made with, and the messages it puts
// aside. Written in PHP because they are PHP's, not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// The run cannot ask the host what mask it was started under, so it
// takes the one a host usually gives and keeps its own from there.
$__masked = 18;
// What each file the run made was made with, under the name it was made
// by. Nothing else is written down, since a file the run did not make
// is one it can ask nobody about.
$__made_with = array();

function umask($mask = null) {
    global $__masked;
    $was = $__masked;
    if ($mask !== null) { $__masked = whole_of($mask) & 511; }
    return $was;
}

// The rights a file is made with: what was asked for, less whatever the
// mask holds back.
function __rights_now($asked) {
    global $__masked;
    return $asked & ~$__masked & 511;
}
// A file is written down as made only the first time it is made; writing
// to one that is already there leaves the rights it has.
function __made($path, $asked) {
    global $__made_with;
    if (array_key_exists($path, $__made_with)) { return null; }
    $__made_with[$path] = __rights_now($asked);
    return null;
}

// What a file may be done with, and by whom. A plain file is said to be
// one, which is the eight thousand and more that stands above the rights
// themselves. A file the run did not make it knows nothing about, so it
// answers with what a plain file is made with.
function fileperms($path) {
    global $__made_with;
    if (!file_exists($path)) {
        __complaint_say(__complaint_word(E_WARNING), "fileperms(): stat failed for " . $path);
        return false;
    }
    if (array_key_exists($path, $__made_with)) { return 32768 + $__made_with[$path]; }
    return 32768 + __rights_now(438);
}

// The run has no clock to ask, so a message put aside is stamped with
// the day the run itself was built, which is the only date it knows.
function __log_stamp() {
    $pieces = array();
    foreach (explode(" ", PHP_BUILD_DATE) as $piece) {
        if ($piece !== "") { $pieces[] = $piece; }
    }
    return str_pad($pieces[1], 2, "0", STR_PAD_LEFT) . "-" . $pieces[0] . "-" . $pieces[2] . " " . $pieces[3] . " UTC";
}
// Text put at the end of a file, which is made where it is not there
// yet, with the rights asked for less what the mask holds back.
function __log_onto($path, $said, $asked) {
    $held = "";
    if (file_exists($path)) {
        $found = __file_read($path);
        if ($found !== false) { $held = $found; }
    }
    __made($path, $asked);
    file_put_contents($path, $held . $said);
    return true;
}
// A message put where the run keeps them. Sending one on as mail is not
// something a run of this kind does, so saying to send one with nowhere
// to send it to is turned down. A message named a file of its own goes
// into it as it stands; one that names none goes, stamped, into the file
// the run was told to keep them in, and where it was told of none there
// is nowhere for it to go and it is quietly let be.
function error_log($message, $sort = 0, $where = null, $headers = null) {
    if ($sort == 1) { return $where !== null; }
    if ($sort == 3) {
        if ($where === null || $where === "") { return false; }
        return __log_onto($where, $message, 438);
    }
    $named = ini_get("error_log");
    if ($named === false || $named === "" || $named === "syslog") { return true; }
    $mode = ini_get("error_log_mode");
    $asked = 420;
    if ($mode !== false && $mode !== "") { $asked = leading_whole($mode); }
    return __log_onto($named, "[" . __log_stamp() . "] " . $message . "\n", $asked);
}
