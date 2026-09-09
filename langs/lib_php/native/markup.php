// Text written so a reader takes it for words and not for markup, and
// read back again. Written in PHP because it is PHP's, not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// Which quotes are written out, and which spelling of markup the text is
// meant for: the two lowest bits say the quotes, the two above them say
// the spelling, and the rest say what to do with a character the charset
// has no place for.
define("ENT_NOQUOTES", 0);
define("ENT_HTML401", 0);
define("ENT_COMPAT", 2);
define("ENT_QUOTES", 3);
define("ENT_IGNORE", 4);
define("ENT_SUBSTITUTE", 8);
define("ENT_XML1", 16);
define("ENT_XHTML", 32);
define("ENT_HTML5", 48);
define("ENT_DISALLOWED", 128);
define("HTML_SPECIALCHARS", 0);
define("HTML_ENTITIES", 1);

// The names markup has for characters, and the characters they name.
// Both are worked out from one list the first time either is wanted,
// since a program that never writes markup should not pay for them.
$__ent_named = null;
$__ent_coded = null;
// Windows-1252 stands where Latin-1 stands, save in the two and thirty
// places above the plain letters, where it keeps marks of its own.
$__ent_windows = null;
$__ent_windows_back = null;

function __ent_tables() {
    global $__ent_named, $__ent_coded, $__ent_windows, $__ent_windows_back;
    if ($__ent_named !== null) { return null; }
    $__ent_named = array();
    $__ent_coded = array();
    $listed =
        "nbsp=160 iexcl=161 cent=162 pound=163 curren=164 yen=165 brvbar=166 " .
        "sect=167 uml=168 copy=169 ordf=170 laquo=171 not=172 shy=173 reg=174 " .
        "macr=175 deg=176 plusmn=177 sup2=178 sup3=179 acute=180 micro=181 " .
        "para=182 middot=183 cedil=184 sup1=185 ordm=186 raquo=187 frac14=188 " .
        "frac12=189 frac34=190 iquest=191 Agrave=192 Aacute=193 Acirc=194 " .
        "Atilde=195 Auml=196 Aring=197 AElig=198 Ccedil=199 Egrave=200 " .
        "Eacute=201 Ecirc=202 Euml=203 Igrave=204 Iacute=205 Icirc=206 " .
        "Iuml=207 ETH=208 Ntilde=209 Ograve=210 Oacute=211 Ocirc=212 " .
        "Otilde=213 Ouml=214 times=215 Oslash=216 Ugrave=217 Uacute=218 " .
        "Ucirc=219 Uuml=220 Yacute=221 THORN=222 szlig=223 agrave=224 " .
        "aacute=225 acirc=226 atilde=227 auml=228 aring=229 aelig=230 " .
        "ccedil=231 egrave=232 eacute=233 ecirc=234 euml=235 igrave=236 " .
        "iacute=237 icirc=238 iuml=239 eth=240 ntilde=241 ograve=242 " .
        "oacute=243 ocirc=244 otilde=245 ouml=246 divide=247 oslash=248 " .
        "ugrave=249 uacute=250 ucirc=251 uuml=252 yacute=253 thorn=254 " .
        "yuml=255 OElig=338 oelig=339 Scaron=352 scaron=353 Yuml=376 fnof=402 " .
        "circ=710 tilde=732 Alpha=913 Beta=914 Gamma=915 Delta=916 " .
        "Epsilon=917 Zeta=918 Eta=919 Theta=920 Iota=921 Kappa=922 Lambda=923 " .
        "Mu=924 Nu=925 Xi=926 Omicron=927 Pi=928 Rho=929 Sigma=931 Tau=932 " .
        "Upsilon=933 Phi=934 Chi=935 Psi=936 Omega=937 alpha=945 beta=946 " .
        "gamma=947 delta=948 epsilon=949 zeta=950 eta=951 theta=952 iota=953 " .
        "kappa=954 lambda=955 mu=956 nu=957 xi=958 omicron=959 pi=960 rho=961 " .
        "sigmaf=962 sigma=963 tau=964 upsilon=965 phi=966 chi=967 psi=968 " .
        "omega=969 thetasym=977 upsih=978 piv=982 ensp=8194 emsp=8195 " .
        "thinsp=8201 zwnj=8204 zwj=8205 lrm=8206 rlm=8207 ndash=8211 " .
        "mdash=8212 lsquo=8216 rsquo=8217 sbquo=8218 ldquo=8220 rdquo=8221 " .
        "bdquo=8222 dagger=8224 Dagger=8225 bull=8226 hellip=8230 permil=8240 " .
        "prime=8242 Prime=8243 lsaquo=8249 rsaquo=8250 oline=8254 frasl=8260 " .
        "euro=8364 image=8465 weierp=8472 real=8476 trade=8482 alefsym=8501 " .
        "larr=8592 uarr=8593 rarr=8594 darr=8595 harr=8596 crarr=8629 " .
        "lArr=8656 uArr=8657 rArr=8658 dArr=8659 hArr=8660 forall=8704 " .
        "part=8706 exist=8707 empty=8709 nabla=8711 isin=8712 notin=8713 " .
        "ni=8715 prod=8719 sum=8721 minus=8722 lowast=8727 radic=8730 " .
        "prop=8733 infin=8734 ang=8736 and=8743 or=8744 cap=8745 cup=8746 " .
        "int=8747 there4=8756 sim=8764 cong=8773 asymp=8776 ne=8800 " .
        "equiv=8801 le=8804 ge=8805 sub=8834 sup=8835 nsub=8836 sube=8838 " .
        "supe=8839 oplus=8853 otimes=8855 perp=8869 sdot=8901 lceil=8968 " .
        "rceil=8969 lfloor=8970 rfloor=8971 lang=9001 rang=9002 loz=9674 " .
        "spades=9824 clubs=9827 hearts=9829 diams=9830 " .
        "";
    foreach (explode(" ", $listed) as $pair) {
        if ($pair === "") { continue; }
        $apart = explode("=", $pair);
        $code = whole_of($apart[1]);
        $__ent_named[$code] = $apart[0];
        $__ent_coded[$apart[0]] = $code;
    }
    $__ent_windows = array(128 => 8364, 130 => 8218, 131 => 402, 132 => 8222, 133 => 8230,
                           134 => 8224, 135 => 8225, 136 => 710, 137 => 8240, 138 => 352,
                           139 => 8249, 140 => 338, 142 => 381, 145 => 8216, 146 => 8217,
                           147 => 8220, 148 => 8221, 149 => 8226, 150 => 8211, 151 => 8212,
                           152 => 732, 153 => 8482, 154 => 353, 155 => 8250, 156 => 339,
                           158 => 382, 159 => 376);
    $__ent_windows_back = array();
    foreach ($__ent_windows as $place => $code) { $__ent_windows_back[$code] = $place; }
    return null;
}

// The charsets the run knows: the one most of the world writes in, the
// older Latin-1, and Windows-1252. A name it does not know it takes for
// the first of them, since that is what a text is likeliest to be.
function __charset_named($named) {
    if ($named === null || $named === "" || !is_string($named)) {
        $named = ini_get("default_charset");
        if ($named === false || $named === "") { return "utf-8"; }
    }
    $held = strtolower($named);
    if ($held === "cp1252" || $held === "windows-1252") { return "cp1252"; }
    if ($held === "iso-8859-1" || $held === "iso8859-1" || $held === "latin1") { return "latin1"; }
    return "utf-8";
}
// The character a piece of text stands for, said as the number the whole
// world knows it by; and the same the other way about, which answers
// with nothing where the charset has no place for the character at all.
function __ent_code_at($letter, $charset) {
    global $__ent_windows;
    $code = ord($letter);
    if ($charset === "cp1252" && $code >= 128 && $code <= 159 && isset($__ent_windows[$code])) {
        return $__ent_windows[$code];
    }
    return $code;
}
function __ent_letter_of($code, $charset) {
    global $__ent_windows_back;
    if ($charset === "utf-8") { return chr($code); }
    if ($charset === "cp1252" && isset($__ent_windows_back[$code])) { return chr($__ent_windows_back[$code]); }
    if ($code < 0 || $code > 255) { return null; }
    if ($charset === "cp1252" && $code >= 128 && $code <= 159) { return null; }
    return chr($code);
}

function __ent_is_sixteens($letter) {
    return is_digit($letter) || (strpos("abcdefABCDEF", $letter) !== false);
}
function __ent_is_name_letter($letter) {
    $code = ord($letter);
    return is_digit($letter) || ($code >= 65 && $code <= 90) || ($code >= 97 && $code <= 122);
}
// The single quote is written as its number where the text is meant for
// HTML as it was first written, and by its name everywhere else.
function __ent_apostrophe($flags) {
    if (($flags & 48) == 0) { return "&#039;"; }
    return "&apos;";
}
// Whether a name is one the run knows: the marks a program may always
// write, and the quotes only where the flags say those are written too.
function __ent_by_name($name, $flags, $all) {
    global $__ent_coded;
    if ($name === "amp") { return 38; }
    if ($name === "lt") { return 60; }
    if ($name === "gt") { return 62; }
    if ($name === "quot") { return ($flags & 2) ? 34 : -1; }
    if ($name === "apos") { return (($flags & 1) && ($flags & 48) != 0) ? 39 : -1; }
    if ($all && isset($__ent_coded[$name])) { return $__ent_coded[$name]; }
    return -1;
}
// How much of the text from here already stands as an entity, or nothing
// where what stands here is a bare ampersand. What is already written
// out is left as it was rather than written out twice over.
function __ent_stands($text, $at, $flags) {
    $size = strlen($text);
    $walk = $at + 1;
    if ($walk < $size && $text[$walk] === "#") {
        $walk = $walk + 1;
        $sixteens = false;
        if ($walk < $size && ($text[$walk] === "x" || $text[$walk] === "X")) { $sixteens = true; $walk = $walk + 1; }
        $from = $walk;
        while ($walk < $size) {
            if ($sixteens && !__ent_is_sixteens($text[$walk])) { break; }
            if (!$sixteens && !is_digit($text[$walk])) { break; }
            $walk = $walk + 1;
        }
        if ($walk == $from || $walk >= $size || $text[$walk] !== ";") { return 0; }
        $said = substr($text, $from, $walk - $from);
        $code = $sixteens ? hexdec($said) : whole_of($said);
        if ($code <= 0 || $code > 1114111) { return 0; }
        return $walk + 1 - $at;
    }
    $from = $walk;
    while ($walk < $size && __ent_is_name_letter($text[$walk])) { $walk = $walk + 1; }
    if ($walk == $from || $walk >= $size || $text[$walk] !== ";") { return 0; }
    if (__ent_by_name(substr($text, $from, $walk - $from), 3, true) < 0) { return 0; }
    return $walk + 1 - $at;
}

// Text written out so that markup reads it as words. Where every name is
// wanted, a character the run has a name for is written under it; where
// only the marks are, the rest of the text stands as it was. A character
// the charset has no place for makes the whole answer nothing, unless
// the flags say to drop it or to put a mark for the missing one.
function __html_written($text, $flags, $charset, $double, $all) {
    global $__ent_named;
    __ent_tables();
    $charset = __charset_named($charset);
    $out = "";
    $at = 0;
    $size = strlen($text);
    while ($at < $size) {
        $letter = $text[$at];
        if ($letter === "&") {
            $stands = $double ? 0 : __ent_stands($text, $at, $flags);
            if ($stands > 0) { $out = $out . substr($text, $at, $stands); $at = $at + $stands; continue; }
            $out = $out . "&amp;";
        } elseif ($letter === "<") {
            $out = $out . "&lt;";
        } elseif ($letter === ">") {
            $out = $out . "&gt;";
        } elseif ($letter === "\"") {
            $out = $out . (($flags & 2) ? "&quot;" : $letter);
        } elseif ($letter === "'") {
            $out = $out . (($flags & 1) ? __ent_apostrophe($flags) : $letter);
        } else {
            $code = __ent_code_at($letter, $charset);
            if ($charset !== "utf-8" && __ent_letter_of($code, $charset) === null) {
                if ($flags & 4) { $at = $at + 1; continue; }
                if ($flags & 8) { $out = $out . chr(65533); $at = $at + 1; continue; }
                return "";
            }
            if ($all && $code > 127 && isset($__ent_named[$code])) {
                $out = $out . "&" . $__ent_named[$code] . ";";
            } else {
                $out = $out . $letter;
            }
        }
        $at = $at + 1;
    }
    return $out;
}

// Text read back from markup. An entity the run has no name for, or one
// whose character the charset has no place for, is left standing as it
// was written, which is what the reference does with it.
function __html_read($text, $flags, $charset, $all) {
    __ent_tables();
    $charset = __charset_named($charset);
    $out = "";
    $at = 0;
    $size = strlen($text);
    while ($at < $size) {
        $letter = $text[$at];
        if ($letter !== "&") { $out = $out . $letter; $at = $at + 1; continue; }
        $walk = $at + 1;
        $code = -1;
        if ($walk < $size && $text[$walk] === "#") {
            $walk = $walk + 1;
            $sixteens = false;
            if ($walk < $size && ($text[$walk] === "x" || $text[$walk] === "X")) { $sixteens = true; $walk = $walk + 1; }
            $from = $walk;
            while ($walk < $size) {
                if ($sixteens && !__ent_is_sixteens($text[$walk])) { break; }
                if (!$sixteens && !is_digit($text[$walk])) { break; }
                $walk = $walk + 1;
            }
            if ($walk > $from && $walk < $size && $text[$walk] === ";") {
                $said = substr($text, $from, $walk - $from);
                $code = $sixteens ? hexdec($said) : whole_of($said);
                if ($code <= 0 || $code > 1114111) { $code = -1; }
                if ($code == 34 && ($flags & 2) == 0) { $code = -1; }
                if ($code == 39 && ($flags & 1) == 0) { $code = -1; }
            }
        } else {
            $from = $walk;
            while ($walk < $size && __ent_is_name_letter($text[$walk])) { $walk = $walk + 1; }
            if ($walk > $from && $walk < $size && $text[$walk] === ";") {
                $code = __ent_by_name(substr($text, $from, $walk - $from), $flags, $all);
            }
        }
        $stands = ($code < 0) ? null : __ent_letter_of($code, $charset);
        if ($stands === null) { $out = $out . $letter; $at = $at + 1; continue; }
        $out = $out . $stands;
        $at = $walk + 1;
    }
    return $out;
}

// The marks markup itself is written with, and nothing else. The flags
// stand at what the reference has them stand at: both quotes written
// out, a character with no place put by, and HTML as it was first
// written.
function htmlspecialchars($text, $flags = 11, $charset = null, $double = true) {
    return __html_written($text, $flags, $charset, $double, false);
}
function htmlspecialchars_decode($text, $flags = 11) {
    return __html_read($text, $flags, null, false);
}
// Every character the run has a name for, written under that name.
function htmlentities($text, $flags = 11, $charset = null, $double = true) {
    return __html_written($text, $flags, $charset, $double, true);
}
function html_entity_decode($text, $flags = 11, $charset = null) {
    return __html_read($text, $flags, $charset, true);
}
// The whole of what either of them would write, laid out as the
// character it stands for against the way that character is written.
function get_html_translation_table($which = 0, $flags = 11, $charset = null) {
    global $__ent_named;
    __ent_tables();
    $charset = __charset_named($charset);
    $out = array();
    if ($flags & 2) { $out["\""] = "&quot;"; }
    if ($flags & 1) { $out["'"] = __ent_apostrophe($flags); }
    $out["<"] = "&lt;";
    $out[">"] = "&gt;";
    $out["&"] = "&amp;";
    if ($which == 1) {
        foreach ($__ent_named as $code => $name) {
            $letter = __ent_letter_of($code, $charset);
            if ($letter !== null) { $out[$letter] = "&" . $name . ";"; }
        }
    }
    return $out;
}
