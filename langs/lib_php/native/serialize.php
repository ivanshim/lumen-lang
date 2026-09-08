// Values written down so that they can be read back, and the fingers
// PHP takes of a piece of text. Written in PHP because they are PHP's
// and not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// A value written down: what kind it is, how long where that matters,
// and what it holds.
function serialize($value) {
    if ($value === null) { return "N;"; }
    if (is_bool($value)) { return "b:" . ($value ? "1" : "0") . ";"; }
    if (is_int($value)) { return "i:" . $value . ";"; }
    if (is_float($value)) { return "d:" . __serialized_real($value) . ";"; }
    if (is_string($value)) { return "s:" . strlen($value) . ":\"" . $value . "\";"; }
    if (is_array($value)) {
        $out = "a:" . count($value) . ":{";
        foreach ($value as $k => $v) { $out = $out . serialize($k) . serialize($v); }
        return $out . "}";
    }
    if (is_object($value)) {
        $named = get_class($value);
        $held = get_object_vars($value);
        $out = "O:" . strlen($named) . ":\"" . $named . "\":" . count($held) . ":{";
        foreach ($held as $k => $v) { $out = $out . serialize((string) $k) . serialize($v); }
        return $out . "}";
    }
    return "N;";
}
function __serialized_real($x) {
    if ($x == (float) (int) $x && $x < 9223372036854775808.0 && $x > -9223372036854775808.0) {
        return strval((int) $x);
    }
    return strval($x);
}

// A value read back from how it was written down. Text that is not a
// value written down that way answers false and says so.
function unserialize($data, $options = array()) {
    $at = 0;
    $held = __unserialized((string) $data, $at);
    if ($held === null) {
        __complaint_say(__complaint_word(E_WARNING), "unserialize(): Error at offset " . $at . " of " . strlen((string) $data) . " bytes");
        return false;
    }
    return $held[0];
}
function __unserialized($text, &$at) {
    if ($at >= strlen($text)) { return null; }
    $mark = $text[$at];
    if ($mark === "N" && substr($text, $at, 2) === "N;") { $at = $at + 2; return array(null); }
    if ($mark === "b") {
        $shut = strpos($text, ";", $at);
        if ($shut === false) { return null; }
        $held = substr($text, $at + 2, $shut - $at - 2) === "1";
        $at = $shut + 1;
        return array($held);
    }
    if ($mark === "i" || $mark === "d") {
        $shut = strpos($text, ";", $at);
        if ($shut === false) { return null; }
        $written = substr($text, $at + 2, $shut - $at - 2);
        $at = $shut + 1;
        return array($mark === "i" ? (int) $written : (float) $written);
    }
    if ($mark === "s") {
        $colon = strpos($text, ":", $at + 2);
        if ($colon === false) { return null; }
        $size = (int) substr($text, $at + 2, $colon - $at - 2);
        $held = substr($text, $colon + 2, $size);
        $at = $colon + 2 + $size + 2;
        return array($held);
    }
    if ($mark === "a" || $mark === "O") {
        return __unserialized_many($text, $at, $mark === "O");
    }
    return null;
}
function __unserialized_many($text, &$at, $a_thing) {
    $named = "stdClass";
    $from = $at + 2;
    if ($a_thing) {
        $colon = strpos($text, ":", $from);
        if ($colon === false) { return null; }
        $size = (int) substr($text, $from, $colon - $from);
        $named = substr($text, $colon + 2, $size);
        $from = $colon + 2 + $size + 2;
    }
    $colon = strpos($text, ":", $from);
    if ($colon === false) { return null; }
    $count = (int) substr($text, $from, $colon - $from);
    $at = $colon + 2;
    $pairs = array();
    $done = 0;
    while ($done < $count) {
        $key = __unserialized($text, $at);
        if ($key === null) { return null; }
        $held = __unserialized($text, $at);
        if ($held === null) { return null; }
        $pairs[$key[0]] = $held[0];
        $done = $done + 1;
    }
    if ($at < strlen($text) && $text[$at] === "}") { $at = $at + 1; }
    if (!$a_thing) { return array($pairs); }
    $thing = new stdClass;
    foreach ($pairs as $k => $v) { $spelt = (string) $k; $thing->$spelt = $v; }
    return array($thing);
}

// ---- the fingers taken of a piece of text ----

// Four numbers of thirty-two bits turned over sixty-four times, which is
// the sum PHP calls md5.
function md5($string, $binary = false) {
    $sines = array(
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391);
    $turns = array(7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,
                   5,9,14,20,5,9,14,20,5,9,14,20,5,9,14,20,
                   4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,
                   6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21);
    $filled = __padded_little($string);
    $a0 = 0x67452301; $b0 = 0xefcdab89; $c0 = 0x98badcfe; $d0 = 0x10325476;
    $block = 0;
    while ($block < strlen($filled)) {
        $words = __words_little($filled, $block);
        $a = $a0; $b = $b0; $c = $c0; $d = $d0;
        $i = 0;
        while ($i < 64) {
            if ($i < 16) { $f = ($b & $c) | ((~$b & 0xFFFFFFFF) & $d); $g = $i; }
            else if ($i < 32) { $f = ($d & $b) | ((~$d & 0xFFFFFFFF) & $c); $g = (5 * $i + 1) % 16; }
            else if ($i < 48) { $f = $b ^ $c ^ $d; $g = (3 * $i + 5) % 16; }
            else { $f = $c ^ ($b | (~$d & 0xFFFFFFFF)); $g = (7 * $i) % 16; }
            $f = ($f + $a + $sines[$i] + $words[$g]) & 0xFFFFFFFF;
            $a = $d; $d = $c; $c = $b;
            $b = ($b + __turned_left($f, $turns[$i])) & 0xFFFFFFFF;
            $i = $i + 1;
        }
        $a0 = ($a0 + $a) & 0xFFFFFFFF;
        $b0 = ($b0 + $b) & 0xFFFFFFFF;
        $c0 = ($c0 + $c) & 0xFFFFFFFF;
        $d0 = ($d0 + $d) & 0xFFFFFFFF;
        $block = $block + 64;
    }
    $out = __little_hex($a0) . __little_hex($b0) . __little_hex($c0) . __little_hex($d0);
    return $binary ? __bytes_of_hex($out) : $out;
}

// Five numbers of thirty-two bits turned over eighty times, which is the
// sum PHP calls sha1.
function sha1($string, $binary = false) {
    $filled = __padded_big($string);
    $h0 = 0x67452301; $h1 = 0xEFCDAB89; $h2 = 0x98BADCFE; $h3 = 0x10325476; $h4 = 0xC3D2E1F0;
    $block = 0;
    while ($block < strlen($filled)) {
        $w = __words_big($filled, $block);
        $i = 16;
        while ($i < 80) {
            $w[$i] = __turned_left($w[$i-3] ^ $w[$i-8] ^ $w[$i-14] ^ $w[$i-16], 1);
            $i = $i + 1;
        }
        $a = $h0; $b = $h1; $c = $h2; $d = $h3; $e = $h4;
        $i = 0;
        while ($i < 80) {
            if ($i < 20) { $f = ($b & $c) | ((~$b & 0xFFFFFFFF) & $d); $k = 0x5A827999; }
            else if ($i < 40) { $f = $b ^ $c ^ $d; $k = 0x6ED9EBA1; }
            else if ($i < 60) { $f = ($b & $c) | ($b & $d) | ($c & $d); $k = 0x8F1BBCDC; }
            else { $f = $b ^ $c ^ $d; $k = 0xCA62C1D6; }
            $held = (__turned_left($a, 5) + $f + $e + $k + $w[$i]) & 0xFFFFFFFF;
            $e = $d; $d = $c; $c = __turned_left($b, 30); $b = $a; $a = $held;
            $i = $i + 1;
        }
        $h0 = ($h0 + $a) & 0xFFFFFFFF;
        $h1 = ($h1 + $b) & 0xFFFFFFFF;
        $h2 = ($h2 + $c) & 0xFFFFFFFF;
        $h3 = ($h3 + $d) & 0xFFFFFFFF;
        $h4 = ($h4 + $e) & 0xFFFFFFFF;
        $block = $block + 64;
    }
    $out = __big_hex($h0) . __big_hex($h1) . __big_hex($h2) . __big_hex($h3) . __big_hex($h4);
    return $binary ? __bytes_of_hex($out) : $out;
}

// The check PHP calls crc32, worked out a bit at a time.
function crc32($string) {
    $held = 0xFFFFFFFF;
    $at = 0;
    while ($at < strlen($string)) {
        $held = $held ^ ord($string[$at]);
        $bit = 0;
        while ($bit < 8) {
            $low = $held & 1;
            $held = $held >> 1;
            if ($low) { $held = $held ^ 0xEDB88320; }
            $bit = $bit + 1;
        }
        $at = $at + 1;
    }
    return $held ^ 0xFFFFFFFF;
}

// The sums a program may ask for by name.
function hash($algo, $data, $binary = false) {
    $named = strtolower($algo);
    if ($named === "md5") { return md5($data, $binary); }
    if ($named === "sha1") { return sha1($data, $binary); }
    if ($named === "crc32b") {
        $held = __big_hex(crc32($data));
        return $binary ? __bytes_of_hex($held) : $held;
    }
    throw new ValueError('hash(): Argument #1 ($algo) must be a valid hashing algorithm');
}
function hash_algos() { return array("md5", "sha1", "crc32b"); }

// A piece of text brought up to a whole number of blocks: a bit set,
// noughts, and how long it was in bits at the end. One writes that
// length with the least of it first and the other with the most.
function __padded_little($string) { return __padded($string, true); }
function __padded_big($string) { return __padded($string, false); }
function __padded($string, $little) {
    $bits = strlen($string) * 8;
    $out = $string . chr(128);
    while (strlen($out) % 64 !== 56) { $out = $out . chr(0); }
    $tail = "";
    $at = 0;
    while ($at < 8) {
        $byte = ($bits >> ($at * 8)) & 255;
        $tail = $little ? $tail . chr($byte) : chr($byte) . $tail;
        $at = $at + 1;
    }
    return $out . $tail;
}
function __words_little($text, $from) {
    $words = array();
    $at = 0;
    while ($at < 16) {
        $b = $from + $at * 4;
        $words[] = ord($text[$b]) | (ord($text[$b+1]) << 8) | (ord($text[$b+2]) << 16) | (ord($text[$b+3]) << 24);
        $at = $at + 1;
    }
    return $words;
}
function __words_big($text, $from) {
    $words = array();
    $at = 0;
    while ($at < 16) {
        $b = $from + $at * 4;
        $words[] = (ord($text[$b]) << 24) | (ord($text[$b+1]) << 16) | (ord($text[$b+2]) << 8) | ord($text[$b+3]);
        $at = $at + 1;
    }
    return $words;
}
function __turned_left($x, $by) {
    $held = $x & 0xFFFFFFFF;
    return ((($held << $by) & 0xFFFFFFFF) | ($held >> (32 - $by))) & 0xFFFFFFFF;
}
function __little_hex($n) {
    $out = "";
    $at = 0;
    while ($at < 4) { $out = $out . str_pad(dechex(($n >> ($at * 8)) & 255), 2, "0", STR_PAD_LEFT); $at = $at + 1; }
    return $out;
}
function __big_hex($n) { return str_pad(dechex($n & 0xFFFFFFFF), 8, "0", STR_PAD_LEFT); }
function __bytes_of_hex($hex) {
    $out = "";
    $at = 0;
    while ($at + 1 < strlen($hex)) { $out = $out . chr(hexdec(substr($hex, $at, 2))); $at = $at + 2; }
    return $out;
}
