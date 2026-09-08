// Patterns, and what PHP does with them. A pattern is read into a tree
// of pieces and matched by trying each piece and going back on what did
// not work out, which is how a pattern with alternatives and counts is
// matched at all.
// Hand-written; scripts/port_examples.py leaves native/ alone.

define("PREG_PATTERN_ORDER", 1);
define("PREG_SET_ORDER", 2);
define("PREG_OFFSET_CAPTURE", 256);
define("PREG_UNMATCHED_AS_NULL", 512);
define("PREG_SPLIT_NO_EMPTY", 1);
define("PREG_SPLIT_DELIM_CAPTURE", 2);
define("PREG_SPLIT_OFFSET_CAPTURE", 4);
define("PREG_NO_ERROR", 0);
define("PREG_INTERNAL_ERROR", 1);

// ---- reading a pattern ----

// A pattern read into a tree, kept so that reading it twice costs once.
function __re_read($pattern) {
    static $known = array();
    if (isset($known[$pattern])) { return $known[$pattern]; }
    $held = __re_pattern($pattern);
    $known[$pattern] = $held;
    return $held;
}

// The whole of a pattern: the mark it is written between, the body, and
// the letters after it that say how it is to be read.
function __re_pattern($pattern) {
    $text = (string) $pattern;
    if (strlen($text) < 2) { return array("bad" => "Empty regular expression"); }
    $open = $text[0];
    $lead = ord($open);
    $alnum = ($lead >= 48 && $lead <= 57) || ($lead >= 65 && $lead <= 90) || ($lead >= 97 && $lead <= 122);
    if ($alnum || $open === "\\" || $lead === 0) {
        return array("bad" => "Delimiter must not be alphanumeric, backslash, or NUL");
    }
    $close = $open;
    if ($open === "(") { $close = ")"; }
    else if ($open === "[") { $close = "]"; }
    else if ($open === "{") { $close = "}"; }
    else if ($open === "<") { $close = ">"; }
    $deep = 0;
    $ended = -1;
    $at = 1;
    while ($at < strlen($text)) {
        $c = $text[$at];
        if ($c === "\\") { $at = $at + 2; continue; }
        if ($open !== $close && $c === $open) { $deep = $deep + 1; }
        else if ($c === $close) {
            if ($deep === 0) { $ended = $at; break; }
            $deep = $deep - 1;
        }
        $at = $at + 1;
    }
    if ($ended < 0) { return array("bad" => "No ending delimiter '" . $open . "' found"); }
    $body = substr($text, 1, $ended - 1);
    $letters = substr($text, $ended + 1);
    $how = array("i" => false, "m" => false, "s" => false, "x" => false, "D" => false, "A" => false);
    $at = 0;
    while ($at < strlen($letters)) {
        $c = $letters[$at];
        if ($c === "u" || $c === "U") { $at = $at + 1; continue; }
        if (!isset($how[$c])) { return array("bad" => "Unknown modifier '" . $c . "'"); }
        $how[$c] = true;
        $at = $at + 1;
    }
    $state = array("at" => 0, "body" => $body, "groups" => 0, "names" => array(), "x" => $how["x"]);
    $tree = __re_alternatives($state);
    if ($state["at"] < strlen($body)) { return array("bad" => "Compilation failed"); }
    return array("tree" => $tree, "how" => $how, "groups" => $state["groups"], "names" => $state["names"]);
}

// One or more sequences with `|` between them.
function __re_alternatives(&$state) {
    $all = array(__re_sequence($state));
    while ($state["at"] < strlen($state["body"]) && $state["body"][$state["at"]] === "|") {
        $state["at"] = $state["at"] + 1;
        $all[] = __re_sequence($state);
    }
    return $all;
}

// One run of pieces, each with whatever count follows it.
function __re_sequence(&$state) {
    $seq = array();
    while ($state["at"] < strlen($state["body"])) {
        $c = $state["body"][$state["at"]];
        if ($c === "|" || $c === ")") { break; }
        $piece = __re_piece($state);
        if ($piece === null) { continue; }
        // Letters turned on or off standing alone hold for the rest of
        // the run they were written in, so what follows is gathered up
        // and read under them.
        if ($piece["k"] === "setflags") {
            $rest = __re_sequence($state);
            $seq[] = array("k" => "group", "kind" => "hold", "n" => 0, "alts" => array($rest),
                           "set" => $piece["set"], "clear" => $piece["clear"]);
            return $seq;
        }
        $seq[] = __re_counted($piece, $state);
    }
    return $seq;
}

// Whatever count follows a piece: any number of times, one or more,
// none or one, or a run written out between braces. A count may be
// asked for as little as will do, or held to once it is met.
function __re_counted($piece, &$state) {
    $body = $state["body"];
    $at = $state["at"];
    if ($at >= strlen($body)) { return $piece; }
    $c = $body[$at];
    $least = 0;
    $most = -1;
    if ($c === "*") { $at = $at + 1; }
    else if ($c === "+") { $least = 1; $at = $at + 1; }
    else if ($c === "?") { $most = 1; $at = $at + 1; }
    else if ($c === "{") {
        $shut = strpos($body, "}", $at);
        if ($shut === false) { return $piece; }
        $inside = substr($body, $at + 1, $shut - $at - 1);
        if (!__re_a_count($inside)) { return $piece; }
        $comma = strpos($inside, ",");
        if ($comma === false) { $least = (int) $inside; $most = $least; }
        else {
            $least = (int) substr($inside, 0, $comma);
            $rest = substr($inside, $comma + 1);
            $most = $rest === "" ? -1 : (int) $rest;
        }
        $at = $shut + 1;
    } else { return $piece; }
    $how = "greedy";
    if ($at < strlen($body) && $body[$at] === "?") { $how = "lazy"; $at = $at + 1; }
    else if ($at < strlen($body) && $body[$at] === "+") { $how = "held"; $at = $at + 1; }
    $state["at"] = $at;
    return array("k" => "count", "of" => $piece, "least" => $least, "most" => $most, "how" => $how);
}
function __re_a_count($inside) {
    if ($inside === "") { return false; }
    $seen = false;
    $at = 0;
    while ($at < strlen($inside)) {
        $c = $inside[$at];
        if ($c >= "0" && $c <= "9") { $seen = true; }
        else if ($c !== ",") { return false; }
        $at = $at + 1;
    }
    return $seen;
}

// One piece: a group, a class, a mark standing for a place rather than
// a character, or a character itself.
function __re_piece(&$state) {
    $body = $state["body"];
    $at = $state["at"];
    $c = $body[$at];
    if ($state["x"]) {
        if ($c === " " || $c === "\t" || $c === "\n" || $c === "\r") { $state["at"] = $at + 1; return null; }
        if ($c === "#") {
            while ($at < strlen($body) && $body[$at] !== "\n") { $at = $at + 1; }
            $state["at"] = $at;
            return null;
        }
    }
    if ($c === "(") { return __re_group($state); }
    if ($c === "[") { return __re_class($state); }
    if ($c === ".") { $state["at"] = $at + 1; return array("k" => "any"); }
    if ($c === "^") { $state["at"] = $at + 1; return array("k" => "bol"); }
    if ($c === "$") { $state["at"] = $at + 1; return array("k" => "eol"); }
    if ($c === "\\") { return __re_escape($state); }
    $state["at"] = $at + 1;
    return array("k" => "char", "c" => $c);
}

// A group: one that is kept, one that is only for holding pieces
// together, or one that looks ahead or behind without taking anything.
function __re_group(&$state) {
    $body = $state["body"];
    $at = $state["at"] + 1;
    $kind = "keep";
    $named = null;
    $number = 0;
    $set = array();
    $clear = array();
    if ($at < strlen($body) && $body[$at] === "?") {
        // `(?i)`, `(?-i)` and `(?i:...)`: letters turned on and off.
        $held = __re_letters($body, $at + 1);
        if ($held !== null) {
            $state["at"] = $held[2] + 1;
            if ($body[$held[2]] === ")") {
                return array("k" => "setflags", "set" => $held[0], "clear" => $held[1]);
            }
            $inside = __re_alternatives($state);
            if ($state["at"] < strlen($body) && $body[$state["at"]] === ")") { $state["at"] = $state["at"] + 1; }
            return array("k" => "group", "kind" => "hold", "n" => 0, "alts" => $inside, "set" => $held[0], "clear" => $held[1]);
        }
        $next = $at + 1 < strlen($body) ? $body[$at + 1] : "";
        if ($next === ":") { $kind = "hold"; $at = $at + 2; }
        else if ($next === "=") { $kind = "ahead"; $at = $at + 2; }
        else if ($next === "!") { $kind = "not_ahead"; $at = $at + 2; }
        else if ($next === "<" && $at + 2 < strlen($body) && $body[$at + 2] === "=") { $kind = "behind"; $at = $at + 3; }
        else if ($next === "<" && $at + 2 < strlen($body) && $body[$at + 2] === "!") { $kind = "not_behind"; $at = $at + 3; }
        else if ($next === ">") { $kind = "once"; $at = $at + 2; }
        else if ($next === "P" || $next === "<" || $next === "'") {
            $from = $next === "P" ? $at + 3 : $at + 2;
            $shut = $next === "'" ? "'" : ">";
            $to = strpos($body, $shut, $from);
            $named = substr($body, $from, $to - $from);
            $at = $to + 1;
        } else { $kind = "hold"; $at = $at + 2; }
    }
    if ($kind === "keep") {
        $state["groups"] = $state["groups"] + 1;
        $number = $state["groups"];
        if ($named !== null) { $state["names"][$named] = $number; }
    }
    $state["at"] = $at;
    $inside = __re_alternatives($state);
    if ($state["at"] < strlen($body) && $body[$state["at"]] === ")") { $state["at"] = $state["at"] + 1; }
    return array("k" => "group", "kind" => $kind, "n" => $number, "alts" => $inside);
}

// The letters a group turns on and off, where that is what it does: the
// ones before a dash are turned on and the ones after it off, and the
// whole must end at a close or a colon. Nothing where the group is some
// other kind.
function __re_letters($body, $at) {
    $known = "imsxU";
    $set = array();
    $clear = array();
    $off = false;
    $seen = false;
    $from = $at;
    while ($from < strlen($body)) {
        $c = $body[$from];
        if ($c === ")" || $c === ":") {
            if (!$seen) { return null; }
            return array($set, $clear, $from);
        }
        if ($c === "-") { $off = true; $seen = true; $from = $from + 1; continue; }
        if (strpos($known, $c) === false) { return null; }
        if ($c !== "U") {
            if ($off) { $clear[$c] = true; } else { $set[$c] = true; }
        }
        $seen = true;
        $from = $from + 1;
    }
    return null;
}

// A run of characters written between brackets, with runs from one to
// another, the whole of it perhaps turned about.
function __re_class(&$state) {
    $body = $state["body"];
    $at = $state["at"] + 1;
    $not = false;
    if ($at < strlen($body) && $body[$at] === "^") { $not = true; $at = $at + 1; }
    $items = array();
    $first = true;
    while ($at < strlen($body)) {
        $c = $body[$at];
        if ($c === "]" && !$first) { $at = $at + 1; break; }
        $first = false;
        if ($c === "[" && $at + 1 < strlen($body) && $body[$at + 1] === ":") {
            $shut = strpos($body, ":]", $at);
            if ($shut !== false) {
                $items[] = array("k" => "posix", "name" => substr($body, $at + 2, $shut - $at - 2));
                $at = $shut + 2;
                continue;
            }
        }
        if ($c === "\\") {
            $held = __re_class_escape($body, $at);
            $at = $held[1];
            $one = $held[0];
            if ($one["k"] === "char" && $at + 1 < strlen($body) && $body[$at] === "-" && $body[$at + 1] !== "]") {
                $to = $body[$at + 1] === "\\" ? __re_class_escape($body, $at + 1) : array(array("k" => "char", "c" => $body[$at + 1]), $at + 2);
                $at = $to[1];
                $items[] = array("k" => "run", "from" => ord($one["c"]), "to" => ord($to[0]["c"]));
                continue;
            }
            $items[] = $one;
            continue;
        }
        if ($at + 2 < strlen($body) && $body[$at + 1] === "-" && $body[$at + 2] !== "]") {
            $to = $body[$at + 2] === "\\" ? __re_class_escape($body, $at + 2) : array(array("k" => "char", "c" => $body[$at + 2]), $at + 3);
            $items[] = array("k" => "run", "from" => ord($c), "to" => ord($to[0]["c"]));
            $at = $to[1];
            continue;
        }
        $items[] = array("k" => "char", "c" => $c);
        $at = $at + 1;
    }
    $state["at"] = $at;
    return array("k" => "class", "not" => $not, "items" => $items);
}

// What a backslash stands for inside a run of characters: a named kind
// of character, or a character written some other way.
function __re_class_escape($body, $at) {
    $c = $at + 1 < strlen($body) ? $body[$at + 1] : "";
    if (strpos("dDwWsShv", $c) !== false && $c !== "") { return array(array("k" => "kind", "which" => $c), $at + 2); }
    return __re_written($body, $at);
}

// A character written with a backslash: the ones with names of their
// own, a number written in base sixteen or eight, and everything else
// standing for itself.
function __re_written($body, $at) {
    $c = $at + 1 < strlen($body) ? $body[$at + 1] : "\\";
    if ($c === "n") { return array(array("k" => "char", "c" => "\n"), $at + 2); }
    if ($c === "r") { return array(array("k" => "char", "c" => "\r"), $at + 2); }
    if ($c === "t") { return array(array("k" => "char", "c" => "\t"), $at + 2); }
    if ($c === "f") { return array(array("k" => "char", "c" => chr(12)), $at + 2); }
    if ($c === "e") { return array(array("k" => "char", "c" => chr(27)), $at + 2); }
    if ($c === "a") { return array(array("k" => "char", "c" => chr(7)), $at + 2); }
    if ($c === "0") { return array(array("k" => "char", "c" => chr(0)), $at + 2); }
    if ($c === "x") {
        $from = $at + 2;
        $held = "";
        if ($from < strlen($body) && $body[$from] === "{") {
            $shut = strpos($body, "}", $from);
            $held = substr($body, $from + 1, $shut - $from - 1);
            return array(array("k" => "char", "c" => chr(hexdec($held) & 255)), $shut + 1);
        }
        while (strlen($held) < 2 && $from < strlen($body) && __re_hex($body[$from])) { $held = $held . $body[$from]; $from = $from + 1; }
        return array(array("k" => "char", "c" => chr($held === "" ? 0 : hexdec($held))), $from);
    }
    return array(array("k" => "char", "c" => $c), $at + 2);
}
function __re_hex($c) {
    return ($c >= "0" && $c <= "9") || ($c >= "a" && $c <= "f") || ($c >= "A" && $c <= "F");
}

// What a backslash stands for outside a run of characters: a named kind
// of character, a place rather than a character, what a kept group
// found before, or a character written some other way.
function __re_escape(&$state) {
    $body = $state["body"];
    $at = $state["at"];
    $c = $at + 1 < strlen($body) ? $body[$at + 1] : "\\";
    if (strpos("dDwWsShv", $c) !== false && $c !== "") {
        $state["at"] = $at + 2;
        return array("k" => "class", "not" => false, "items" => array(array("k" => "kind", "which" => $c)));
    }
    if ($c === "b") { $state["at"] = $at + 2; return array("k" => "edge", "not" => false); }
    if ($c === "B") { $state["at"] = $at + 2; return array("k" => "edge", "not" => true); }
    if ($c === "A") { $state["at"] = $at + 2; return array("k" => "bos"); }
    if ($c === "z") { $state["at"] = $at + 2; return array("k" => "eos"); }
    if ($c === "Z") { $state["at"] = $at + 2; return array("k" => "eos_nl"); }
    if ($c === "G") { $state["at"] = $at + 2; return array("k" => "bos"); }
    if ($c === "Q") {
        $shut = strpos($body, "\\E", $at + 2);
        $upto = $shut === false ? strlen($body) : $shut;
        $held = substr($body, $at + 2, $upto - $at - 2);
        $state["at"] = $shut === false ? strlen($body) : $shut + 2;
        $seq = array();
        $i = 0;
        while ($i < strlen($held)) { $seq[] = array("k" => "char", "c" => $held[$i]); $i = $i + 1; }
        return array("k" => "group", "kind" => "hold", "n" => 0, "alts" => array($seq));
    }
    if ($c >= "1" && $c <= "9") {
        $held = "";
        $from = $at + 1;
        while ($from < strlen($body) && $body[$from] >= "0" && $body[$from] <= "9") { $held = $held . $body[$from]; $from = $from + 1; }
        $state["at"] = $from;
        return array("k" => "again", "n" => (int) $held);
    }
    if ($c === "k") {
        $from = $at + 2;
        $shut = $from < strlen($body) && $body[$from] === "{" ? "}" : ($from < strlen($body) && $body[$from] === "'" ? "'" : ">");
        $to = strpos($body, $shut, $from);
        $state["at"] = $to + 1;
        return array("k" => "again_named", "name" => substr($body, $from + 1, $to - $from - 1));
    }
    $held = __re_written($body, $at);
    $state["at"] = $held[1];
    return $held[0];
}

// ---- matching ----

// Whether a character is one of a named kind.
function __re_of_kind($c, $which) {
    $code = ord($c);
    $digit = $code >= 48 && $code <= 57;
    $word = $digit || ($code >= 65 && $code <= 90) || ($code >= 97 && $code <= 122) || $code === 95;
    $blank = $c === " " || $c === "\t" || $c === "\n" || $c === "\r" || $code === 11 || $code === 12;
    if ($which === "d") { return $digit; }
    if ($which === "D") { return !$digit; }
    if ($which === "w") { return $word; }
    if ($which === "W") { return !$word; }
    if ($which === "s") { return $blank; }
    if ($which === "S") { return !$blank; }
    if ($which === "h") { return $c === " " || $c === "\t"; }
    if ($which === "v") { return $c === "\n" || $c === "\r" || $code === 11 || $code === 12; }
    return false;
}
function __re_of_posix($c, $name) {
    $code = ord($c);
    $upper = $code >= 65 && $code <= 90;
    $lower = $code >= 97 && $code <= 122;
    $digit = $code >= 48 && $code <= 57;
    if ($name === "alpha") { return $upper || $lower; }
    if ($name === "digit") { return $digit; }
    if ($name === "alnum") { return $upper || $lower || $digit; }
    if ($name === "space") { return __re_of_kind($c, "s"); }
    if ($name === "upper") { return $upper; }
    if ($name === "lower") { return $lower; }
    if ($name === "punct") { return $code > 32 && $code < 127 && !$upper && !$lower && !$digit; }
    if ($name === "xdigit") { return __re_hex($c); }
    if ($name === "word") { return __re_of_kind($c, "w"); }
    if ($name === "print") { return $code >= 32 && $code < 127; }
    if ($name === "graph") { return $code > 32 && $code < 127; }
    if ($name === "cntrl") { return $code < 32 || $code === 127; }
    if ($name === "blank") { return $c === " " || $c === "\t"; }
    return false;
}

// Whether a character stands in a run of them.
function __re_in_class($node, $c, $fold) {
    $found = false;
    foreach ($node["items"] as $item) {
        if ($item["k"] === "char") {
            if ($fold ? strtolower($item["c"]) === strtolower($c) : $item["c"] === $c) { $found = true; }
        } else if ($item["k"] === "run") {
            $code = ord($c);
            if ($code >= $item["from"] && $code <= $item["to"]) { $found = true; }
            if ($fold) {
                $other = ord($c >= "a" && $c <= "z" ? strtoupper($c) : strtolower($c));
                if ($other >= $item["from"] && $other <= $item["to"]) { $found = true; }
            }
        } else if ($item["k"] === "kind") {
            if (__re_of_kind($c, $item["which"])) { $found = true; }
        } else if ($item["k"] === "posix") {
            if (__re_of_posix($c, $item["name"])) { $found = true; }
        }
    }
    return $node["not"] ? !$found : $found;
}

function __re_word_at($text, $at) {
    if ($at < 0 || $at >= strlen($text)) { return false; }
    return __re_of_kind($text[$at], "w");
}

// The heart of it: match the piece standing at `$at` of `$seq`, and
// then whatever is left to match after it. What is left is carried as a
// list of places to go back to, innermost last. Nothing where the match
// does not work out; otherwise where it ended and what the kept groups
// found.
function __re_go($seq, $at, $rest, $text, $pos, $caps, $how) {
    while ($at >= count($seq)) {
        if (count($rest) === 0) { return array($pos, $caps); }
        $frame = $rest[count($rest) - 1];
        $rest = array_slice($rest, 0, count($rest) - 1);
        $seq = $frame[0];
        $at = $frame[1];
    }
    $node = $seq[$at];
    $k = $node["k"];
    $size = strlen($text);
    if ($k === "char") {
        if ($pos >= $size) { return false; }
        $one = $text[$pos];
        $same = $how["i"] ? strtolower($one) === strtolower($node["c"]) : $one === $node["c"];
        if (!$same) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos + 1, $caps, $how);
    }
    if ($k === "any") {
        if ($pos >= $size) { return false; }
        if (!$how["s"] && $text[$pos] === "\n") { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos + 1, $caps, $how);
    }
    if ($k === "class") {
        if ($pos >= $size) { return false; }
        if (!__re_in_class($node, $text[$pos], $how["i"])) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos + 1, $caps, $how);
    }
    if ($k === "bol") {
        $at_start = $pos === 0 || ($how["m"] && $text[$pos - 1] === "\n");
        if (!$at_start) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($k === "eol") {
        $at_end = $pos === $size || ($pos === $size - 1 && $text[$pos] === "\n" && !$how["D"]) || ($how["m"] && $pos < $size && $text[$pos] === "\n");
        if (!$at_end) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($k === "bos") {
        if ($pos !== 0) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($k === "eos") {
        if ($pos !== $size) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($k === "eos_nl") {
        if (!($pos === $size || ($pos === $size - 1 && $text[$pos] === "\n"))) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($k === "edge") {
        $an_edge = __re_word_at($text, $pos - 1) !== __re_word_at($text, $pos);
        if ($an_edge === $node["not"]) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($k === "again" || $k === "again_named") {
        $n = $k === "again" ? $node["n"] : (isset($how["names"][$node["name"]]) ? $how["names"][$node["name"]] : 0);
        if (!isset($caps[$n]) || $caps[$n] === null) { return false; }
        $held = substr($text, $caps[$n][0], $caps[$n][1] - $caps[$n][0]);
        $seen = substr($text, $pos, strlen($held));
        $same = $how["i"] ? strtolower($seen) === strtolower($held) : $seen === $held;
        if (!$same) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos + strlen($held), $caps, $how);
    }
    if ($k === "shut") {
        $caps[$node["n"]] = array($node["s"], $pos);
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($k === "group") { return __re_group_go($node, $seq, $at, $rest, $text, $pos, $caps, $how); }
    if ($k === "count") { return __re_count_go($node, $seq, $at, $rest, $text, $pos, $caps, $how); }
    return false;
}

// A group: each alternative is tried in turn, with what is left to
// match after the group carried along behind it. One that looks ahead
// or behind takes nothing and only says whether it is there.
function __re_group_go($node, $seq, $at, $rest, $text, $pos, $caps, $how) {
    $kind = $node["kind"];
    if ($kind === "ahead" || $kind === "not_ahead") {
        $found = false;
        foreach ($node["alts"] as $alt) {
            if (__re_go($alt, 0, array(), $text, $pos, $caps, $how) !== false) { $found = true; }
        }
        if ($found === ($kind === "not_ahead")) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    if ($kind === "behind" || $kind === "not_behind") {
        $found = false;
        $from = $pos;
        while ($from >= 0 && !$found) {
            foreach ($node["alts"] as $alt) {
                $held = __re_go($alt, 0, array(), $text, $from, $caps, $how);
                if ($held !== false && $held[0] === $pos) { $found = true; }
            }
            $from = $from - 1;
        }
        if ($found === ($kind === "not_behind")) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    // Letters a group turns on or off hold inside it and nowhere else.
    if (isset($node["set"]) || isset($node["clear"])) {
        $within = $how;
        if (isset($node["set"])) { foreach ($node["set"] as $letter => $on) { $within[$letter] = true; } }
        if (isset($node["clear"])) { foreach ($node["clear"] as $letter => $off) { $within[$letter] = false; } }
        foreach ($node["alts"] as $alt) {
            $held = __re_go($alt, 0, array(), $text, $pos, $caps, $within);
            if ($held === false) { continue; }
            $onward = __re_go($seq, $at + 1, $rest, $text, $held[0], $held[1], $how);
            if ($onward !== false) { return $onward; }
        }
        return false;
    }
    $after = $rest;
    $after[] = array($seq, $at + 1);
    foreach ($node["alts"] as $alt) {
        $inner = $alt;
        if ($kind === "keep") { $inner[] = array("k" => "shut", "n" => $node["n"], "s" => $pos); }
        if ($kind === "once") {
            $held = __re_go($inner, 0, array(), $text, $pos, $caps, $how);
            if ($held === false) { continue; }
            $onward = __re_go($seq, $at + 1, $rest, $text, $held[0], $held[1], $how);
            if ($onward !== false) { return $onward; }
            continue;
        }
        $held = __re_go($inner, 0, $after, $text, $pos, $caps, $how);
        if ($held !== false) { return $held; }
    }
    return false;
}

// A count: the piece taken as many times as it will go, or as few, and
// gone back on until the rest of the pattern works out. A count held to
// takes as many as it can and never gives one back.
function __re_count_go($node, $seq, $at, $rest, $text, $pos, $caps, $how) {
    $least = $node["least"];
    $most = $node["most"];
    $of = array($node["of"]);
    if ($node["how"] === "held") {
        $taken = 0;
        $where = $pos;
        $held_caps = $caps;
        while ($most < 0 || $taken < $most) {
            $step = __re_go($of, 0, array(), $text, $where, $held_caps, $how);
            if ($step === false || $step[0] === $where) { break; }
            $where = $step[0];
            $held_caps = $step[1];
            $taken = $taken + 1;
        }
        if ($taken < $least) { return false; }
        return __re_go($seq, $at + 1, $rest, $text, $where, $held_caps, $how);
    }
    return __re_count_from($node, $of, 0, $seq, $at, $rest, $text, $pos, $caps, $how);
}

// So many taken already: either one more, or on to the rest, whichever
// the count asks to be tried first.
function __re_count_from($node, $of, $taken, $seq, $at, $rest, $text, $pos, $caps, $how) {
    $least = $node["least"];
    $most = $node["most"];
    $may_take = $most < 0 || $taken < $most;
    $may_stop = $taken >= $least;
    if ($node["how"] === "lazy" && $may_stop) {
        $held = __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
        if ($held !== false) { return $held; }
    }
    if ($may_take) {
        $step = __re_go($of, 0, array(), $text, $pos, $caps, $how);
        if ($step !== false && !($step[0] === $pos && $taken >= $least)) {
            $held = __re_count_from($node, $of, $taken + 1, $seq, $at, $rest, $text, $step[0], $step[1], $how);
            if ($held !== false) { return $held; }
        }
    }
    if ($node["how"] !== "lazy" && $may_stop) {
        return __re_go($seq, $at + 1, $rest, $text, $pos, $caps, $how);
    }
    return false;
}

// The first place at or after an offset where a pattern matches, and
// what its kept groups found. Nothing where it matches nowhere.
function __re_first($read, $text, $offset) {
    $how = $read["how"];
    $how["names"] = $read["names"];
    $from = $offset;
    while ($from <= strlen($text)) {
        $caps = array();
        foreach ($read["tree"] as $alt) {
            $inner = $alt;
            $inner[] = array("k" => "shut", "n" => 0, "s" => $from);
            $held = __re_go($inner, 0, array(), $text, $from, $caps, $how);
            if ($held !== false) { return $held[1]; }
        }
        if ($how["A"]) { return null; }
        $from = $from + 1;
    }
    return null;
}

// ---- what a program asks for ----

function preg_last_error() { return PREG_NO_ERROR; }
function preg_last_error_msg() { return "No error"; }

// The characters a pattern gives a meaning of their own to, written so
// that they stand for themselves.
function preg_quote($str, $delimiter = null) {
    $marks = ".\\+*?[^]$(){}=!<>|:-#/";
    $out = "";
    $at = 0;
    while ($at < strlen($str)) {
        $c = $str[$at];
        if (strpos($marks, $c) !== false || ($delimiter !== null && $delimiter !== "" && $c === $delimiter)) { $out = $out . "\\"; }
        if (ord($c) === 0) { $out = $out . "\\000"; } else { $out = $out . $c; }
        $at = $at + 1;
    }
    return $out;
}

// Whether a pattern matches, and what it found where it did.
function preg_match($pattern, $subject, &$matches = null, $flags = 0, $offset = 0) {
    $read = __re_read($pattern);
    if (isset($read["bad"])) {
        __complaint_say(__complaint_word(E_WARNING), "preg_match(): " . $read["bad"]);
        $matches = array();
        return false;
    }
    $text = (string) $subject;
    $found = __re_first($read, $text, $offset < 0 ? strlen($text) + $offset : $offset);
    if ($found === null) { $matches = array(); return 0; }
    $matches = __re_gathered($found, $text, $read, $flags);
    return 1;
}

// Every place a pattern matches, gathered by group or by match.
function preg_match_all($pattern, $subject, &$matches = null, $flags = PREG_PATTERN_ORDER, $offset = 0) {
    $read = __re_read($pattern);
    if (isset($read["bad"])) {
        __complaint_say(__complaint_word(E_WARNING), "preg_match_all(): " . $read["bad"]);
        $matches = array();
        return false;
    }
    $text = (string) $subject;
    $at = $offset < 0 ? strlen($text) + $offset : $offset;
    $sets = array();
    $held = 0;
    while ($at <= strlen($text)) {
        $found = __re_first($read, $text, $at);
        if ($found === null) { break; }
        $sets[] = __re_gathered($found, $text, $read, $flags & ~PREG_SET_ORDER & ~PREG_PATTERN_ORDER);
        $held = $held + 1;
        $at = $found[0][1] > $found[0][0] ? $found[0][1] : $found[0][0] + 1;
    }
    if ($flags & PREG_SET_ORDER) { $matches = $sets; return $held; }
    $by_group = array();
    $names = __re_named_places($read);
    $g = 0;
    while ($g <= $read["groups"]) {
        $column = array();
        foreach ($sets as $set) { $column[] = isset($set[$g]) ? $set[$g] : ""; }
        if (isset($names[$g])) { $by_group[$names[$g]] = $column; }
        $by_group[$g] = $column;
        $g = $g + 1;
    }
    $matches = $by_group;
    return $held;
}

function __re_named_places($read) {
    $out = array();
    foreach ($read["names"] as $named => $number) { $out[$number] = $named; }
    return $out;
}

// What a match found, as PHP hands it over: the whole of it first, then
// each kept group, with a group's name standing beside its number.
function __re_gathered($caps, $text, $read, $flags) {
    $names = __re_named_places($read);
    $out = array();
    $last = 0;
    $g = 0;
    while ($g <= $read["groups"]) {
        if (isset($caps[$g]) && $caps[$g] !== null) { $last = $g; }
        $g = $g + 1;
    }
    $g = 0;
    while ($g <= $last) {
        $there = isset($caps[$g]) && $caps[$g] !== null;
        $held = $there ? substr($text, $caps[$g][0], $caps[$g][1] - $caps[$g][0]) : ($flags & PREG_UNMATCHED_AS_NULL ? null : "");
        $where = $there ? $caps[$g][0] : -1;
        $one = $flags & PREG_OFFSET_CAPTURE ? array($held, $where) : $held;
        if (isset($names[$g])) { $out[$names[$g]] = $one; }
        $out[$g] = $one;
        $g = $g + 1;
    }
    return $out;
}

// Every match replaced by something else, either written out with the
// kept groups standing in it or worked out by a routine.
function preg_replace($pattern, $replacement, $subject, $limit = -1, &$count = null) {
    return __re_replaced($pattern, $replacement, $subject, $limit, $count, false);
}
function preg_replace_callback($pattern, $callback, $subject, $limit = -1, &$count = null, $flags = 0) {
    return __re_replaced($pattern, $callback, $subject, $limit, $count, true);
}
function __re_replaced($pattern, $with, $subject, $limit, &$count, $by_routine) {
    $count = 0;
    if (is_array($subject)) {
        $out = array();
        foreach ($subject as $k => $one) {
            $held = 0;
            $out[$k] = __re_replaced($pattern, $with, $one, $limit, $held, $by_routine);
            $count = $count + $held;
        }
        return $out;
    }
    $patterns = is_array($pattern) ? $pattern : array($pattern);
    $text = (string) $subject;
    $at_pattern = 0;
    foreach ($patterns as $key => $one) {
        $answer = $by_routine ? $with : (is_array($with) ? (isset($with[$key]) ? $with[$key] : "") : $with);
        $held = 0;
        $text = __re_one_replaced($one, $answer, $text, $limit, $held, $by_routine);
        if ($text === null) { return null; }
        $count = $count + $held;
        $at_pattern = $at_pattern + 1;
    }
    return $text;
}
function __re_one_replaced($pattern, $with, $text, $limit, &$count, $by_routine) {
    $read = __re_read($pattern);
    if (isset($read["bad"])) {
        __complaint_say(__complaint_word(E_WARNING), "preg_replace(): " . $read["bad"]);
        return null;
    }
    $out = "";
    $at = 0;
    $done = 0;
    while ($at <= strlen($text) && ($limit < 0 || $done < $limit)) {
        $found = __re_first($read, $text, $at);
        if ($found === null) { break; }
        $from = $found[0][0];
        $to = $found[0][1];
        $out = $out . substr($text, $at, $from - $at);
        $held = __re_gathered($found, $text, $read, 0);
        $out = $out . ($by_routine ? (string) $with($held) : __re_filled($with, $held));
        $done = $done + 1;
        if ($to === $from) {
            if ($from < strlen($text)) { $out = $out . $text[$from]; }
            $at = $from + 1;
        } else { $at = $to; }
    }
    if ($at <= strlen($text)) { $out = $out . substr($text, $at); }
    $count = $done;
    return $out;
}

// What a replacement says, with what the groups found written into it:
// `$1`, `${1}` and `\1` all stand for the first kept group.
function __re_filled($with, $held) {
    $said = (string) $with;
    $out = "";
    $at = 0;
    while ($at < strlen($said)) {
        $c = $said[$at];
        if (($c === "$" || $c === "\\") && $at + 1 < strlen($said)) {
            $from = $at + 1;
            $braced = $c === "$" && $said[$from] === "{";
            if ($braced) { $from = $from + 1; }
            $digits = "";
            while ($from < strlen($said) && strlen($digits) < 2 && $said[$from] >= "0" && $said[$from] <= "9") {
                $digits = $digits . $said[$from];
                $from = $from + 1;
            }
            if ($digits !== "") {
                if ($braced) {
                    if ($from < strlen($said) && $said[$from] === "}") { $from = $from + 1; }
                    else { $out = $out . $c; $at = $at + 1; continue; }
                }
                $n = (int) $digits;
                $out = $out . (isset($held[$n]) ? $held[$n] : "");
                $at = $from;
                continue;
            }
        }
        if ($c === "\\" && $at + 1 < strlen($said) && $said[$at + 1] === "\\") { $out = $out . "\\"; $at = $at + 2; continue; }
        $out = $out . $c;
        $at = $at + 1;
    }
    return $out;
}

// Text cut apart wherever a pattern matches.
function preg_split($pattern, $subject, $limit = -1, $flags = 0) {
    $read = __re_read($pattern);
    if (isset($read["bad"])) {
        __complaint_say(__complaint_word(E_WARNING), "preg_split(): " . $read["bad"]);
        return false;
    }
    $text = (string) $subject;
    $out = array();
    $at = 0;
    $from = 0;
    $held = 0;
    while ($at <= strlen($text)) {
        if ($limit > 0 && $held >= $limit - 1) { break; }
        $found = __re_first($read, $text, $at);
        if ($found === null) { break; }
        $cut = $found[0][0];
        $to = $found[0][1];
        if ($to === $cut && $cut === $from) { $at = $at + 1; continue; }
        $piece = substr($text, $from, $cut - $from);
        if (!($flags & PREG_SPLIT_NO_EMPTY) || $piece !== "") {
            $out[] = $flags & PREG_SPLIT_OFFSET_CAPTURE ? array($piece, $from) : $piece;
            $held = $held + 1;
        }
        if ($flags & PREG_SPLIT_DELIM_CAPTURE) {
            $g = 1;
            while ($g <= $read["groups"]) {
                if (isset($found[$g]) && $found[$g] !== null) {
                    $one = substr($text, $found[$g][0], $found[$g][1] - $found[$g][0]);
                    if (!($flags & PREG_SPLIT_NO_EMPTY) || $one !== "") {
                        $out[] = $flags & PREG_SPLIT_OFFSET_CAPTURE ? array($one, $found[$g][0]) : $one;
                    }
                }
                $g = $g + 1;
            }
        }
        $from = $to;
        $at = $to === $cut ? $to + 1 : $to;
    }
    $piece = substr($text, $from);
    if (!($flags & PREG_SPLIT_NO_EMPTY) || $piece !== "") {
        $out[] = $flags & PREG_SPLIT_OFFSET_CAPTURE ? array($piece, $from) : $piece;
    }
    return $out;
}

// The items of an array a pattern matches, or those it does not.
function preg_grep($pattern, $array, $flags = 0) {
    $out = array();
    foreach ($array as $k => $v) {
        $held = null;
        $found = preg_match($pattern, (string) $v, $held) === 1;
        if ($found !== ($flags !== 0)) { $out[$k] = $v; }
    }
    return $out;
}
