// The streams a program has open, written in PHP because they are
// PHP's, not the kernel's. Hand-written; scripts/port_examples.py
// leaves native/ alone.

// Streams: what a program has opened, and where in each it is reading.
// A stream is held in the pieces it was written in rather than as one
// long piece of text, because laying the whole of a stream out again for
// every piece put at its end would cost more the longer it grew, and a
// program that writes a file a thousand times over would pay for that a
// thousand times. A handle is the number a stream is filed under and the
// place in it the next read begins at. A program may hold a handle, hand
// it on and give it back; nothing else knows what it is.
$__streams = array();
$__stream_next = 1;

define("SEEK_SET", 0);
define("SEEK_CUR", 1);
define("SEEK_END", 2);
define("STREAM_FILTER_READ", 1);
define("STREAM_FILTER_WRITE", 2);
define("STREAM_FILTER_ALL", 3);

// Pieces put together into one. Two are joined at a time and the answers
// joined again, so that the text is copied over a few times rather than
// once for every piece there is.
function __pieces_joined($pieces) {
    $many = count($pieces);
    if ($many == 0) { return ""; }
    while ($many > 1) {
        $next = array();
        $at = 0;
        while ($at + 1 < $many) {
            $next[] = $pieces[$at] . $pieces[$at + 1];
            $at = $at + 2;
        }
        if ($at < $many) { $next[] = $pieces[$at]; }
        $pieces = $next;
        $many = count($pieces);
    }
    return $pieces[0];
}

function __stream_open($holds, $named, $writes, $says) {
    return __stream_open_pieces(array($holds), $named, $writes, $says);
}
function __stream_open_pieces($pieces, $named, $writes, $says) {
    global $__streams, $__stream_next;
    $handle = $__stream_next;
    $__stream_next = $handle + 1;
    $size = 0;
    foreach ($pieces as $piece) { $size = $size + strlen($piece); }
    $__streams[$handle] = array("pieces" => $pieces, "size" => $size, "at" => 0,
                                "named" => $named, "writes" => $writes, "says" => $says,
                                "kept" => false, "filters" => array());
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
    __complaint_say(__complaint_word(E_WARNING), $said . '(): Argument #1 ($stream) must be of type resource, ' . gettype($handle) . " given");
    return false;
}
// Where a program is reading, or nothing where a seek took it off the
// stream and it has not been put back.
function __stream_place($stream) {
    if ($stream["at"] === null) { return null; }
    return $stream["at"];
}
// The pieces last written are kept loose, in a heap of their own, rather
// than set down beside the stream straight away: setting one down means
// writing the stream's whole list of pieces out again, and a program
// that writes a file a thousand times over would have that done a
// thousand times. Only one stream has a loose heap at a time, and the
// heap is put where it belongs the moment anything asks after a stream.
$__tail_of = null;
$__tail = array();
$__tail_wide = 0;
function __stream_settle() {
    global $__streams, $__tail, $__tail_of, $__tail_wide;
    if ($__tail_of === null) { return null; }
    $handle = $__tail_of;
    $__tail_of = null;
    if (array_key_exists($handle, $__streams)) {
        $stream = $__streams[$handle];
        $pieces = $stream["pieces"];
        foreach ($__tail as $piece) { $pieces[] = $piece; }
        $stream["pieces"] = $pieces;
        $stream["size"] = $stream["size"] + $__tail_wide;
        $stream["at"] = $stream["at"] + $__tail_wide;
        $__streams[$handle] = $stream;
    }
    $__tail = array();
    $__tail_wide = 0;
    return null;
}
// The stream a handle stands for, with everything written to it already
// where it belongs. Everything that asks what a stream holds asks this.
function __stream_ready($handle) {
    __stream_settle();
    return __stream_at($handle);
}
// One thing about a stream, written where the program can see it again.
function __stream_put($handle, $name, $value) {
    global $__streams;
    __stream_settle();
    if (!array_key_exists($handle, $__streams)) { return null; }
    $stream = $__streams[$handle];
    $stream[$name] = $value;
    $__streams[$handle] = $stream;
    return null;
}
// Another piece at the end of a stream, which is all that writing at the
// end need do.
function __stream_add($handle, $said) {
    global $__tail, $__tail_of, $__tail_wide;
    if ($__tail_of !== $handle) {
        __stream_settle();
        $__tail_of = $handle;
    }
    $__tail[] = $said;
    $__tail_wide = $__tail_wide + strlen($said);
    return null;
}
// A whole stream laid out afresh, which is what anything but writing at
// the end comes to.
function __stream_set($handle, $holds) {
    global $__streams;
    $stream = $__streams[$handle];
    $stream["pieces"] = array($holds);
    $stream["size"] = strlen($holds);
    $__streams[$handle] = $stream;
}
// What a stream holds, all of it, put together once and kept that way.
function __stream_text($handle) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return ""; }
    if (count($stream["pieces"]) > 1) {
        __stream_set($handle, __pieces_joined($stream["pieces"]));
        $stream = __stream_ready($handle);
    }
    if (count($stream["pieces"]) == 0) { return ""; }
    return $stream["pieces"][0];
}
// What stands in a stream from one place onward, handed back as the
// pieces it is held in, so that a piece wanted whole is never taken
// apart and put back together again.
function __stream_gathered($handle, $at, $length) {
    $stream = __stream_ready($handle);
    $upto = $stream["size"];
    if ($length !== null && $at + $length < $upto) { $upto = $at + $length; }
    $out = array();
    if ($upto <= $at) { return $out; }
    $from = 0;
    foreach ($stream["pieces"] as $piece) {
        $wide = strlen($piece);
        $end = $from + $wide;
        if ($end > $at && $from < $upto) {
            $begins = max($at, $from);
            $ends = min($end, $upto);
            if ($begins == $from && $ends == $end) { $out[] = $piece; }
            else { $out[] = substr($piece, $begins - $from, $ends - $begins); }
        }
        $from = $end;
        if ($from >= $upto) { break; }
    }
    return $out;
}
function __pieces_wide($pieces) {
    $wide = 0;
    foreach ($pieces as $piece) { $wide = $wide + strlen($piece); }
    return $wide;
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
    $pieces = array("");
    $at = 0;
    if (strpos($mode, "w") === false && strpos($mode, "x") === false) {
        $held = __file_read($path);
        if ($held === false) {
            if (strpos($mode, "a") === false) {
                __complaint_say(__complaint_word(E_WARNING), "fopen(" . $path . "): Failed to open stream: No such file or directory");
                return false;
            }
            $held = "";
        }
        $pieces = __wrote_pieces_of($path, $held);
        if (strpos($mode, "a") !== false) { $at = strlen($held); }
    }
    $handle = __stream_open_pieces($pieces, $path, $writes, false);
    if ($at > 0) { __stream_put($handle, "at", $at); }
    return $handle;
}
// The last file the run wrote out it still has in hand, in the pieces it
// was written in. A file opened for reading is opened on those pieces
// where what stands on the disk is still what went out, since taking a
// long text apart again is work the run has already done once. Only the
// last one is kept: a run that writes a hundred files should not be made
// to hold all hundred of them.
$__wrote_named = null;
$__wrote_pieces = null;
$__wrote_whole = null;
function __wrote_down($named, $pieces, $whole) {
    global $__wrote_named, $__wrote_pieces, $__wrote_whole;
    $__wrote_named = $named;
    $__wrote_pieces = $pieces;
    $__wrote_whole = $whole;
    return null;
}
function __wrote_pieces_of($named, $held) {
    global $__wrote_named, $__wrote_pieces, $__wrote_whole;
    if ($__wrote_named === $named && $__wrote_whole === $held) { return $__wrote_pieces; }
    return array($held);
}
// What a stream holds, put back where it came from. A stream that was
// opened on a file is written out; one held only in the run is not.
function __stream_flush($handle) {
    global $__streams;
    $stream = __stream_ready($handle);
    if ($stream === null) { return false; }
    if (!$stream["writes"]) { return true; }
    if (substr($stream["named"], 0, 6) === "php://") { return true; }
    $pieces = $stream["pieces"];
    $whole = __stream_text($handle);
    file_put_contents($stream["named"], $whole);
    __wrote_down($stream["named"], $pieces, $whole);
    return true;
}
function fclose($handle) {
    global $__streams;
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("fclose", $handle); }
    __stream_flush($handle);
    foreach ($stream["filters"] as $which) { __filter_forget($which); }
    unset($__streams[$handle]);
    return true;
}
function fflush($handle) { return __stream_flush($handle); }
function feof($handle) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("feof", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return true; }
    return $at >= $stream["size"];
}
function ftell($handle) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("ftell", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    return $at;
}
// A seek names a place from the beginning, from where the reading is,
// or from the end. A place before the beginning is no place at all: the
// stream loses its own, and keeps none until it is wound back.
function fseek($handle, $offset, $whence = 0) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return -1; }
    $from = 0;
    if ($whence === SEEK_CUR) {
        $from = __stream_place($stream);
        if ($from === null) { return -1; }
    }
    if ($whence === SEEK_END) { $from = $stream["size"]; }
    $wanted = $from + $offset;
    if ($wanted < 0) {
        __stream_put($handle, "at", null);
        return -1;
    }
    __stream_put($handle, "at", $wanted);
    return 0;
}
function rewind($handle) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return false; }
    __stream_put($handle, "at", 0);
    return true;
}
function fgetc($handle) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("fgetc", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    if ($at >= $stream["size"]) { return false; }
    __stream_put($handle, "at", $at + 1);
    return __stream_through($handle, __stream_gathered($handle, $at, 1), STREAM_FILTER_READ);
}
// The reading moves on by what was read before any filter has had it,
// since a filter changes the text and not the stream.
function fread($handle, $length) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("fread", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    if ($at >= $stream["size"]) { return ""; }
    $pieces = __stream_gathered($handle, $at, $length);
    __stream_put($handle, "at", $at + __pieces_wide($pieces));
    return __stream_through($handle, $pieces, STREAM_FILTER_READ);
}
// A line, the newline that ends it and all: as much as is left where
// nothing ends it, and false where nothing at all is left.
function fgets($handle, $length = null) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("fgets", $handle); }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $holds = __stream_text($handle);
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
    return __stream_through($handle, array(substr($holds, $at, $walk - $at)), STREAM_FILTER_READ);
}
// Writing at the end of a stream only puts another piece there; writing
// anywhere else has to lay the whole of it out again. What the program
// asked to write is what it is told was written, whatever a filter made
// of the text on its way in.
function fwrite($handle, $said, $length = null) {
    global $__tail_of, $__tail_wide;
    // The heap of loose pieces is left where it is: a write at the end
    // only adds to it, and nothing here asks what the stream holds.
    $stream = __stream_at($handle);
    if ($stream === null) { return __stream_amiss("fwrite", $handle); }
    if ($length !== null) { $said = substr($said, 0, $length); }
    $asked = strlen($said);
    $said = __stream_through($handle, array($said), STREAM_FILTER_WRITE);
    if ($stream["says"]) {
        echo $said;
        return $asked;
    }
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $size = $stream["size"];
    if ($__tail_of === $handle) { $at = $at + $__tail_wide; $size = $size + $__tail_wide; }
    if ($at == $size) {
        __stream_add($handle, $said);
        return $asked;
    }
    $holds = __stream_text($handle);
    $end = strlen($holds);
    if ($at > $end) { $holds = $holds . str_repeat("\0", $at - $end); }
    $holds = substr($holds, 0, $at) . $said . substr($holds, $at + strlen($said));
    __stream_set($handle, $holds);
    __stream_put($handle, "at", $at + strlen($said));
    return $asked;
}
function fputs($handle, $said, $length = null) { return fwrite($handle, $said, $length); }
function stream_get_contents($handle, $length = null, $offset = -1) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("stream_get_contents", $handle); }
    if ($offset >= 0) { fseek($handle, $offset, SEEK_SET); }
    $stream = __stream_ready($handle);
    $at = __stream_place($stream);
    if ($at === null) { return false; }
    $wanted = null;
    if ($length !== null && $length >= 0) { $wanted = $length; }
    $pieces = __stream_gathered($handle, $at, $wanted);
    __stream_put($handle, "at", $at + __pieces_wide($pieces));
    return __stream_through($handle, $pieces, STREAM_FILTER_READ);
}
function fpassthru($handle) {
    $said = stream_get_contents($handle);
    if ($said === false) { return false; }
    echo $said;
    return strlen($said);
}
function ftruncate($handle, $size) {
    $stream = __stream_ready($handle);
    if ($stream === null) { return false; }
    $holds = __stream_text($handle);
    if (strlen($holds) > $size) {
        __stream_set($handle, substr($holds, 0, $size));
    } else {
        __stream_set($handle, $holds . str_repeat("\0", $size - strlen($holds)));
    }
    return true;
}

// The filters a program has put on its streams. A filter is the number
// it is filed under, which is what the program is handed and what it
// hands back to take the filter off again.
$__filters = array();
$__filter_next = 1;

// The filters this run knows how to make: text turned thirteen places
// along the alphabet, and text written large or small.
function stream_get_filters() {
    return array("string.rot13", "string.toupper", "string.tolower");
}
function __filter_known($named) {
    return in_array($named, stream_get_filters());
}
// A filter put on a stream, at the end of what it already has or before
// them. Which way it works is asked of the stream where the program does
// not say: a stream that is written to has it on the way out, and one
// that is read from on the way in.
function __filter_added($handle, $named, $way, $first) {
    global $__filters, $__filter_next, $__streams;
    $stream = __stream_ready($handle);
    if ($stream === null) { return __stream_amiss("stream_filter_append", $handle); }
    if (!__filter_known($named)) {
        __complaint_say(__complaint_word(E_WARNING), 'stream_filter_append(): Unable to locate filter "' . $named . '"');
        return false;
    }
    if ($way == 0) { $way = $stream["writes"] ? STREAM_FILTER_ALL : STREAM_FILTER_READ; }
    $which = $__filter_next;
    $__filter_next = $which + 1;
    $__filters[$which] = array("of" => $handle, "named" => $named, "way" => $way);
    $held = $stream["filters"];
    if ($first) { array_unshift($held, $which); } else { $held[] = $which; }
    __stream_put($handle, "filters", $held);
    return $which;
}
function stream_filter_append($handle, $named, $way = 0, $params = null) {
    return __filter_added($handle, $named, $way, false);
}
function stream_filter_prepend($handle, $named, $way = 0, $params = null) {
    return __filter_added($handle, $named, $way, true);
}
function __filter_forget($which) {
    global $__filters;
    unset($__filters[$which]);
    return true;
}
// A filter taken off the stream it was put on. What the stream has
// already handed out it keeps; only what is read or written from here on
// goes by without it.
function stream_filter_remove($which) {
    global $__filters;
    if (!is_int($which) || !array_key_exists($which, $__filters)) {
        __complaint_say(__complaint_word(E_WARNING), 'stream_filter_remove(): Argument #1 ($stream_filter) must be of type resource, ' . gettype($which) . " given");
        return false;
    }
    $filter = $__filters[$which];
    $stream = __stream_ready($filter["of"]);
    if ($stream !== null) {
        $held = array();
        foreach ($stream["filters"] as $each) {
            if ($each !== $which) { $held[] = $each; }
        }
        __stream_put($filter["of"], "filters", $held);
    }
    return __filter_forget($which);
}
// Text on its way through whatever filters the stream has for the way it
// is going, each in the order they were put on. The text goes through in
// the pieces it was gathered in, since a filter works on a piece at a
// time and a piece is small enough to work on.
function __stream_through($handle, $pieces, $way) {
    global $__filters;
    $stream = __stream_at($handle);
    if ($stream === null) { return __pieces_joined($pieces); }
    foreach ($stream["filters"] as $which) {
        $filter = $__filters[$which];
        if (($filter["way"] & $way) == 0) { continue; }
        $through = array();
        foreach ($pieces as $piece) { $through[] = __filter_over($filter["named"], $piece); }
        $pieces = $through;
    }
    return __pieces_joined($pieces);
}
// The letters of the alphabet turned thirteen places along, or written
// large, or written small; anything else stands as it is.
function __filter_letter($named, $letter) {
    $code = ord($letter);
    if ($named === "string.toupper") {
        if ($code >= 97 && $code <= 122) { return chr($code - 32); }
        return $letter;
    }
    if ($named === "string.tolower") {
        if ($code >= 65 && $code <= 90) { return chr($code + 32); }
        return $letter;
    }
    if ($code >= 65 && $code <= 90) { return chr(65 + (($code - 65 + 13) % 26)); }
    if ($code >= 97 && $code <= 122) { return chr(97 + (($code - 97 + 13) % 26)); }
    return $letter;
}
// A filter works a character at a time, which is what a filter of this
// kind is. The piece of text it was last given is kept beside what it
// made of it, since a file written over and over from the one piece
// hands a filter the same text again and again, and turning it afresh
// each time is work already done.
$__filter_was = null;
$__filter_made = null;
function __filter_over($named, $text) {
    global $__filter_was, $__filter_made;
    $asked = $named . "\n" . $text;
    if ($__filter_was === $asked) { return $__filter_made; }
    $out = array();
    $at = 0;
    $size = strlen($text);
    while ($at < $size) {
        $out[] = __filter_letter($named, $text[$at]);
        $at = $at + 1;
    }
    $made = __pieces_joined($out);
    $__filter_was = $asked;
    $__filter_made = $made;
    return $made;
}
