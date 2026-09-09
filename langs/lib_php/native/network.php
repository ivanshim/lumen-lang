// Reaching another host over the web, and reading back what it answers.
// A name beginning http:// or https:// is not a file on the disk, and a
// program that opens one is asking for a request to be made and the
// answer to stand where the file's contents would have stood. What the
// program wants that request to say it puts in a context, under the
// wrapper's own name, and stream_context_create hands it along.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// THE PLACE LEFT FOR THE KERNEL.
//
// Everything below is written and stands ready. One thing is missing,
// and it is the one thing the library cannot do for itself: opening a
// connection to a host and a port, writing on it and reading back what
// comes. That is the kernel's to give, as reading a file is, and it is
// asked for here under the name __net_ask, in the same manner as
// __file_read is asked for.
//
// The builtin wanted, spelled as the others are spelled:
//
//   label      ext.builtin.net.ask
//   name       __net_ask            (what langs/php.json binds it to)
//   handed     $host    the host to reach, as bytes
//              $port    the port to reach it on, a whole number
//              $sent    what to write on the connection once it is
//                       made, whole, as bytes
//              $seconds how long to wait, both for the connection and
//                       for what comes back; nought waits as long as
//                       the host will
//   answers    the bytes that came back, read until the far end closes
//              the connection; false where the connection could not be
//              made, could not be written on, or was broken before
//              anything came back at all
//
// One writing and one reading is the whole of it, which is enough for a
// request that says Connection: close — and this implementation's own
// server answers nothing else. Until the label is bound, __net_ask is
// not a routine this run has, __net_reachable answers false, and every
// http:// name is turned away in the words the reference uses for a
// wrapper it has not got.

// Whether the run can reach another host at all. Answered by asking
// after the routine rather than by any setting, so that binding the
// label is the whole of what turns this on.
function __net_reachable() {
    return function_exists("__net_ask");
}

// How far a text runs before the first of a set of letters, and the
// whole of it where none of them is in it. Written here because taking
// an address apart wants it and the library has no such routine.
function __url_upto($text, $any) {
    $wide = strlen($text);
    $at = 0;
    while ($at < $wide) {
        if (strpos($any, $text[$at]) !== false) { return $at; }
        $at = $at + 1;
    }
    return $wide;
}
// Where a letter stands last in a text, and nothing where it does not
// stand in it at all.
function __url_last_of($text, $letter) {
    $at = strlen($text) - 1;
    while ($at >= 0) {
        if ($text[$at] === $letter) { return $at; }
        $at = $at - 1;
    }
    return false;
}

// A web address taken apart into the pieces a request is built from: the
// scheme, the host, the port it is to be reached on, and what is asked
// for. A port left unsaid is the one the scheme is usually reached on.
// A name that is not a web address at all answers nothing.
function __url_apart($url) {
    $at = strpos($url, "://");
    if ($at === false) { return null; }
    $scheme = strtolower(substr($url, 0, $at));
    $rest = substr($url, $at + 3);
    $ends = __url_upto($rest, "/?#");
    $where = substr($rest, 0, $ends);
    $asked = substr($rest, $ends);
    if ($asked === "" || $asked === false) { $asked = "/"; }
    $user = "";
    $mark = __url_last_of($where, "@");
    if ($mark !== false) {
        $user = substr($where, 0, $mark);
        $where = substr($where, $mark + 1);
    }
    $port = $scheme === "https" ? 443 : 80;
    $said = false;
    // A host written in square brackets is written that way because it
    // holds colons of its own, and the port is what follows the closing
    // bracket.
    if (substr($where, 0, 1) === "[") {
        $shut = strpos($where, "]");
        if ($shut === false) { return null; }
        $host = substr($where, 1, $shut - 1);
        $tail = substr($where, $shut + 1);
        if (substr($tail, 0, 1) === ":") { $port = (int)substr($tail, 1); $said = true; }
    } else {
        $colon = strpos($where, ":");
        if ($colon === false) { $host = $where; }
        else {
            $host = substr($where, 0, $colon);
            $port = (int)substr($where, $colon + 1);
            $said = true;
        }
    }
    if ($host === "") { return null; }
    return array("scheme" => $scheme, "host" => $host, "port" => $port,
                 "port_said" => $said, "asked" => $asked, "user" => $user);
}

// What the program told the http wrapper, and nothing where it told it
// nothing. Both wrappers are asked after under their own names, since a
// program setting options for https means them for the request it is
// about to make and not for some other.
function __http_told($scheme, $context) {
    if ($context === null) { return array(); }
    if (!__context_is($context)) { return array(); }
    $options = stream_context_get_options($context);
    if (!is_array($options)) { return array(); }
    if (array_key_exists($scheme, $options) && is_array($options[$scheme])) { return $options[$scheme]; }
    return array();
}
function __http_told_at($told, $name, $otherwise) {
    if (array_key_exists($name, $told)) { return $told[$name]; }
    return $otherwise;
}

// The headers the program wrote, as a list of lines. A program may write
// them as one text with a break between each, or hand a list of them
// outright; either way what comes back is the lines, with the empty ones
// dropped.
function __http_header_lines($said) {
    $lines = array();
    if (is_array($said)) {
        foreach ($said as $one) {
            if (!is_string($one)) { continue; }
            foreach (__http_header_lines($one) as $line) { $lines[] = $line; }
        }
        return $lines;
    }
    if (!is_string($said)) { return $lines; }
    $said = str_replace("\r\n", "\n", $said);
    $said = str_replace("\r", "\n", $said);
    foreach (explode("\n", $said) as $line) {
        $line = trim($line);
        if ($line !== "") { $lines[] = $line; }
    }
    return $lines;
}
// Whether the program already wrote a header of its own by that name, so
// that the wrapper does not write it a second time.
function __http_header_written($lines, $name) {
    $name = strtolower($name) . ":";
    $wide = strlen($name);
    foreach ($lines as $line) {
        if (strtolower(substr($line, 0, $wide)) === $name) { return true; }
    }
    return false;
}

// The request itself, written out as it goes on the connection. The host
// is named first, as the reference names it, then what the wrapper must
// add for the request to be understood — that the connection is to be
// closed when the answer has been given, and how long the body is — and
// last of all whatever the program wrote for itself. That is the order
// the reference writes them in, and a program that counts the lines of
// its own request should find them where it looks for them.
function __http_request_bytes($apart, $told) {
    $method = strtoupper((string)__http_told_at($told, "method", "GET"));
    if ($method === "") { $method = "GET"; }
    $version = __http_told_at($told, "protocol_version", "1.1");
    $version = is_string($version) ? $version : (string)(float)$version;
    if ($version === "1") { $version = "1.0"; }
    $content = __http_told_at($told, "content", "");
    if (!is_string($content)) { $content = ""; }
    $asked = $apart["asked"];
    if (__http_told_at($told, "request_fulluri", false)) {
        $asked = $apart["scheme"] . "://" . $apart["host"];
        if ($apart["port_said"]) { $asked = $asked . ":" . $apart["port"]; }
        $asked = $asked . $apart["asked"];
    }
    $lines = __http_header_lines(__http_told_at($told, "header", ""));
    $out = $method . " " . $asked . " HTTP/" . $version . "\r\n";
    if (!__http_header_written($lines, "Host")) {
        $host = $apart["host"];
        if ($apart["port_said"]) { $host = $host . ":" . $apart["port"]; }
        $out = $out . "Host: " . $host . "\r\n";
    }
    if (!__http_header_written($lines, "Connection")) {
        $out = $out . "Connection: close\r\n";
    }
    $agent = __http_told_at($told, "user_agent", null);
    if (is_string($agent) && $agent !== "" && !__http_header_written($lines, "User-Agent")) {
        $out = $out . "User-Agent: " . $agent . "\r\n";
    }
    if ($content !== "" && !__http_header_written($lines, "Content-Length")) {
        $out = $out . "Content-Length: " . strlen($content) . "\r\n";
    }
    foreach ($lines as $line) { $out = $out . $line . "\r\n"; }
    return $out . "\r\n" . $content;
}

// What came back, taken apart into the lines the host wrote before the
// break and the body after it. An answer with no break in it at all is
// all headers and no body, which is what a host that says nothing sends.
function __http_answer_apart($answer) {
    $end = strpos($answer, "\r\n\r\n");
    $wide = 4;
    if ($end === false) {
        $end = strpos($answer, "\n\n");
        $wide = 2;
    }
    if ($end === false) { return array(__http_header_lines($answer), ""); }
    $head = substr($answer, 0, $end);
    $body = substr($answer, $end + $wide);
    return array(__http_header_lines($head), $body === false ? "" : $body);
}
// The value a header carries, or nothing where the host did not write it.
function __http_answer_header($lines, $name) {
    $name = strtolower($name) . ":";
    $wide = strlen($name);
    foreach ($lines as $line) {
        if (strtolower(substr($line, 0, $wide)) === $name) { return trim(substr($line, $wide)); }
    }
    return null;
}
// What the host made of the request, as the number it wrote on its first
// line; nought where it wrote nothing that reads as one.
function __http_answer_standing($lines) {
    if (count($lines) == 0) { return 0; }
    $parts = explode(" ", $lines[0]);
    if (count($parts) < 2) { return 0; }
    return (int)$parts[1];
}
// A body a host sent in pieces, put back together. Each piece says in
// figures of sixteen how long it is, and a piece of no length at all is
// the last. A body that was not sent this way is answered as it came.
function __http_answer_whole($lines, $body) {
    $how = __http_answer_header($lines, "Transfer-Encoding");
    if ($how === null || strtolower($how) !== "chunked") { return $body; }
    $out = "";
    $at = 0;
    $wide = strlen($body);
    while ($at < $wide) {
        $end = strpos($body, "\r\n", $at);
        if ($end === false) { break; }
        $said = trim(substr($body, $at, $end - $at));
        $semi = strpos($said, ";");
        if ($semi !== false) { $said = substr($said, 0, $semi); }
        $length = hexdec($said);
        $at = $end + 2;
        if ($length <= 0) { break; }
        $out = $out . substr($body, $at, $length);
        $at = $at + $length + 2;
    }
    return $out;
}

// The words the reference uses where a run has no wrapper for a scheme,
// and where it has one but was told not to use it.
function __http_wrapper_off($said, $url, $scheme) {
    if (!__net_reachable()) {
        __complaint_say(__complaint_word(E_WARNING), $said . "(" . $url . "): Failed to open stream: no suitable wrapper could be found");
        return false;
    }
    __complaint_say(__complaint_word(E_WARNING), $said . "(): " . $scheme . ":// wrapper is disabled in the server configuration by allow_url_fopen=0");
    __complaint_say(__complaint_word(E_WARNING), $said . "(" . $url . "): Failed to open stream: no suitable wrapper could be found");
    return false;
}
// Whether a name is one this wrapper answers for.
function __http_named($path) {
    if (!is_string($path)) { return false; }
    $head = strtolower(substr($path, 0, 8));
    return substr($head, 0, 7) === "http://" || $head === "https://";
}
// Whether the run was told it may open a name that is not a file.
function __url_fopen_allowed() {
    $said = ini_get("allow_url_fopen");
    return !($said === "" || $said === "0" || $said === false);
}

// The whole of a request: the address taken apart, what the program told
// the wrapper written out, the connection made and the answer read back
// and taken apart again. Where the host answers with a place to go
// instead and the program asked to be sent on, the new place is asked
// after in turn, as many times as the program allowed.
//
// Answers the lines the host wrote and the body it sent, or false where
// the request could not be made at all — having said why in the words
// the reference says it in.
function __http_fetched($said, $url, $context) {
    $apart = __url_apart($url);
    if ($apart === null) {
        __complaint_say(__complaint_word(E_WARNING), $said . "(" . $url . "): Failed to open stream: Invalid argument");
        return false;
    }
    if (!__net_reachable() || !__url_fopen_allowed()) {
        return __http_wrapper_off($said, $url, $apart["scheme"]);
    }
    // Nothing here speaks the wrapping a secure connection needs; a
    // program asking for one is told so rather than quietly given a
    // connection with nothing wrapped around it.
    if ($apart["scheme"] === "https") {
        __complaint_say(__complaint_word(E_WARNING), $said . "(): Unable to find the wrapper \"https\" - did you forget to enable it when you configured PHP?");
        __complaint_say(__complaint_word(E_WARNING), $said . "(" . $url . "): Failed to open stream: no suitable wrapper could be found");
        return false;
    }
    $told = __http_told($apart["scheme"], $context);
    $seconds = (int)__http_told_at($told, "timeout", 0);
    $left = (int)__http_told_at($told, "max_redirects", 20);
    $sent = __http_request_bytes($apart, $told);
    $answer = __net_ask($apart["host"], $apart["port"], $sent, $seconds);
    if ($answer === false) {
        __complaint_say(__complaint_word(E_WARNING), $said . "(" . $url . "): Failed to open stream: Connection refused");
        return false;
    }
    $apiece = __http_answer_apart($answer);
    $lines = $apiece[0];
    $body = __http_answer_whole($lines, $apiece[1]);
    $standing = __http_answer_standing($lines);
    if ($standing >= 300 && $standing < 400 && $left > 1) {
        $where = __http_answer_header($lines, "Location");
        if ($where !== null && $where !== "") {
            $told["max_redirects"] = $left - 1;
            $onward = __http_where_next($url, $where);
            // A place sent on to is asked after plainly: nothing of the
            // first request's body goes with it.
            $told["content"] = "";
            $made = stream_context_create(array($apart["scheme"] => $told));
            return __http_fetched($said, $onward, $made);
        }
    }
    if ($standing >= 400 && !__http_told_at($told, "ignore_errors", false)) {
        $first = count($lines) > 0 ? $lines[0] : "HTTP/1.1 " . $standing;
        __complaint_say(__complaint_word(E_WARNING), $said . "(" . $url . "): Failed to open stream: " . $first);
        return false;
    }
    return array($lines, $body);
}
// A place a host sends a program on to, read against the place it was
// asked from: one written whole stands as it is, one beginning with a
// stroke is asked of the same host, and anything else is asked for
// beside what was asked for before.
function __http_where_next($url, $where) {
    if (strpos($where, "://") !== false) { return $where; }
    $apart = __url_apart($url);
    if ($apart === null) { return $where; }
    $host = $apart["scheme"] . "://" . $apart["host"];
    if ($apart["port_said"]) { $host = $host . ":" . $apart["port"]; }
    if (substr($where, 0, 1) === "/") { return $host . $where; }
    $asked = $apart["asked"];
    $cut = __url_last_of($asked, "/");
    $under = $cut === false ? "/" : substr($asked, 0, $cut + 1);
    return $host . $under . $where;
}
