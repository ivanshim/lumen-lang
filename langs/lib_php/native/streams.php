// The streams a program has open, written in PHP because they are
// PHP's, not the kernel's. Hand-written; scripts/port_examples.py
// leaves native/ alone.

// Streams: what a program has opened, and where in each it is reading.
// A stream is held whole, so a handle is the number it is filed under
// and the place in it the next read begins at. A program may hold a
// handle, hand it on and give it back; nothing else knows what it is.
$__streams = array();
$__stream_next = 1;

define("SEEK_SET", 0);
define("SEEK_CUR", 1);
define("SEEK_END", 2);

function __stream_open($holds, $named, $writes, $says) {
    global $__streams, $__stream_next;
    $handle = $__stream_next;
    $__stream_next = $handle + 1;
    $__streams[$handle] = array("holds" => $holds, "at" => 0, "named" => $named, "writes" => $writes, "says" => $says, "kept" => false);
    return $handle;
}
// The stream a handle stands for, or nothing where it stands for none.
function __stream_at($handle) {
    global $__streams;
    if (!is_int($handle)) { return null; }
    if (!array_key_exists($handle, $__streams)) { return null; }
    return $__streams[$handle];
}
function __stream_amiss($said, $handle) {
    __complain(E_WARNING, $said . '(): Argument #1 ($stream) must be of type resource, ' . gettype($handle) . " given");
    return false;
}
// Where a program is reading, or nothing where a seek took it off the
// stream and it has not been put back.
function __stream_place($stream) {
    if ($stream["at"] === null) { return null; }
    return $stream["at"];
}

function fopen($path, $mode = "r") {
    global $__request_body;
    // The body of the request the run was started with is a stream a
    // program may open as often as it likes: each opening reads it from
    // its beginning, since it is held as it came rather than drawn from.
    if ($path === "php://input") {
        $body = "";
        if (is_string($__request_body)) { $body = $__request_body; }
        return __stream_open($body, $path, false, false);
    }
    if ($path === "php://memory" || $path === "php://temp" || substr($path, 0, 12) === "php://temp/m") {
        return __stream_open("", $path, true, false);
    }
    if ($path === "php://stdout" || $path === "php://output" || $path === "php://stderr") {
        return __stream_open("", $path, true, true);
    }
    $writes = (strpos($mode, "w") !== false) || (strpos($mode, "a") !== false) || (strpos($mode, "x") !== false) || (strpos($mode, "+") !== false);
    $holds = "";
    $at = 0;
    if (strpos($mode, "w") === false && strpos($mode, "x") === false) {
        $held = __file_read($path);
        if ($held === false) {
            if (strpos($mode, "a") === false) {
                __complain(E_WARNING, "fopen(" . $path . "): Failed to open stream: No such file or directory");
                return false;
            }
            $held = "";
        }
        $holds = $held;
        if (strpos($mode, "a") !== false) { $at = strlen($holds); }
    }
    $handle = __stream_open($holds, $path, $writes, false);
    if ($at > 0) { __stream_put($handle, "at", $at); }
    return $handle;
}
// One thing about a stream, written where the program can see it again.
function __stream_put($handle, $name, $value) {
    global $__streams;
    $stream = $__streams[$handle];
    $stream[$name] = $value;
    $__streams[$handle] = $stream;
}
// What a stream holds, put back where it came from. A stream that was
// opened on a file is written out; one held only in the run is not.
function __stream_flush($handle) {
    global $__streams;
    $stream = __stream_at($handle);
    if ($stream === null) { return false; }
    if (!$stream["writes"]) { return true; }
    if (substr($stream["named"], 0, 6) === "php://") { return true; }
    file_put_contents($stream["named"], $stream["holds"]);
    return true;
}
function fclose($handle) {
    global $__streams;
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("fclose", $handle); }
    __stream_flush($handle);
    unset($__streams[$handle]);
    return true;
}
function fflush($handle) { return __stream_flush($handle); }
function feof($handle) {
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("feof", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return true; }
    return $at >= strlen($stream["holds"]);
}
function ftell($handle) {
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("ftell", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    return $at;
}
// A seek names a place from the beginning, from where the reading is,
// or from the end. A place before the beginning is no place at all: the
// stream loses its own, and keeps none until it is wound back.
function fseek($handle, $offset, $whence = 0) {
    $stream = __stream_at($handle);
    if ($stream === null) { return -1; }
    $from = 0;
    if ($whence === SEEK_CUR) {
        $from = __stream_place($stream);
        if ($from === null) { return -1; }
    }
    if ($whence === SEEK_END) { $from = strlen($stream["holds"]); }
    $wanted = $from + $offset;
    if ($wanted < 0) {
        __stream_put($handle, "at", null);
        return -1;
    }
    __stream_put($handle, "at", $wanted);
    return 0;
}
function rewind($handle) {
    $stream = __stream_at($handle);
    if ($stream === null) { return false; }
    __stream_put($handle, "at", 0);
    return true;
}
function fgetc($handle) {
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("fgetc", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $holds = $stream["holds"];
    if ($at >= strlen($holds)) { return false; }
    __stream_put($handle, "at", $at + 1);
    return $holds[$at];
}
function fread($handle, $length) {
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("fread", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $holds = $stream["holds"];
    if ($at >= strlen($holds)) { return ""; }
    $said = substr($holds, $at, $length);
    __stream_put($handle, "at", $at + strlen($said));
    return $said;
}
// A line, the newline that ends it and all: as much as is left where
// nothing ends it, and false where nothing at all is left.
function fgets($handle, $length = null) {
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("fgets", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $holds = $stream["holds"];
    $end = strlen($holds);
    if ($at >= $end) { return false; }
    $stop = $end;
    if ($length !== null && $at + $length - 1 < $stop) { $stop = $at + $length - 1; }
    $walk = $at;
    while ($walk < $stop) {
        if ($holds[$walk] === "\n") {
            $walk = $walk + 1;
            break;
        }
        $walk = $walk + 1;
    }
    __stream_put($handle, "at", $walk);
    return substr($holds, $at, $walk - $at);
}
function fwrite($handle, $said, $length = null) {
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("fwrite", $handle); }
    if ($length !== null) { $said = substr($said, 0, $length); }
    if ($stream["says"]) {
        echo $said;
        return strlen($said);
    }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $holds = $stream["holds"];
    $end = strlen($holds);
    if ($at > $end) { $holds = $holds . str_repeat("\0", $at - $end); }
    $holds = substr($holds, 0, $at) . $said . substr($holds, $at + strlen($said));
    __stream_put($handle, "holds", $holds);
    __stream_put($handle, "at", $at + strlen($said));
    return strlen($said);
}
function fputs($handle, $said, $length = null) { return fwrite($handle, $said, $length); }
function stream_get_contents($handle, $length = null, $offset = -1) {
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("stream_get_contents", $handle); }
    if ($offset >= 0) { fseek($handle, $offset, SEEK_SET); }
    $stream = __stream_at($handle);
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $holds = $stream["holds"];
    $said = substr($holds, $at);
    if ($length !== null && $length >= 0) { $said = substr($said, 0, $length); }
    __stream_put($handle, "at", $at + strlen($said));
    return $said;
}
function fpassthru($handle) {
    $said = stream_get_contents($handle);
    if ($said === false) { return false; }
    echo $said;
    return strlen($said);
}
function ftruncate($handle, $size) {
    $stream = __stream_at($handle);
    if ($stream === null) { return false; }
    $holds = $stream["holds"];
    if (strlen($holds) > $size) {
        __stream_put($handle, "holds", substr($holds, 0, $size));
    } else {
        __stream_put($handle, "holds", $holds . str_repeat("\0", $size - strlen($holds)));
    }
    return true;
}
