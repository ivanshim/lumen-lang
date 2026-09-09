// Dates and times as PHP hands them to a program: the words a person
// writes a time in, and the classes a program holds one in. Written in
// PHP because they are PHP's and not the kernel's.
//
// The run keeps no table of the world's zones, so a zone written as an
// offset from the meridian stands at that offset and every zone named
// after a place stands at the meridian itself.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// The zone a run counts its dates in, by name. Only the name is kept.
function __zone_named($set) {
    static $held = "UTC";
    if ($set !== null) { $held = $set; }
    return $held;
}

// How many seconds a zone stands ahead of the meridian. A zone written
// as an offset says so itself; a zone named after a place stands at the
// meridian, the run having no table to look it up in.
function __zone_offset($named) {
    $text = (string) $named;
    if ($text === "" || $text === "UTC" || $text === "GMT" || $text === "Z") { return 0; }
    $held = array();
    if (preg_match('/^([+-])(\d{1,2}):?(\d{2})?$/', $text, $held)) {
        $hours = (int) $held[1 + 1];
        $minutes = isset($held[3]) && $held[3] !== "" ? (int) $held[3] : 0;
        $seconds = $hours * 3600 + $minutes * 60;
        return $held[1] === "-" ? -$seconds : $seconds;
    }
    return 0;
}

// ---- the words a person writes a time in ----

// A time worked out from the words it is written in, counted from the
// start of the year the clock counts from. False where the words say no
// time at all.
function strtotime($datetime, $baseTimestamp = null) {
    $base = $baseTimestamp === null ? __clock() : $baseTimestamp;
    $said = strtolower(trim((string) $datetime));
    if ($said === "") { return false; }
    if ($said[0] === "@") {
        $rest = substr($said, 1);
        if (!is_numeric($rest)) { return false; }
        return (int) $rest;
    }
    $m = __moment($base);
    $held = array("year" => $m["year"], "mon" => $m["mon"], "mday" => $m["mday"],
                  "hours" => $m["hours"], "minutes" => $m["minutes"], "seconds" => $m["seconds"],
                  "offset" => 0, "shift" => array(0, 0, 0), "weekday" => null, "which" => 0, "read" => false);
    $at = 0;
    while ($at < strlen($said)) {
        if ($said[$at] === " " || $said[$at] === "," || $said[$at] === "\t") { $at = $at + 1; continue; }
        $step = __strtotime_piece($said, $at, $held);
        if ($step === false) { return false; }
        $at = $step;
        $held["read"] = true;
    }
    if (!$held["read"]) { return false; }
    return __strtotime_gathered($held);
}

// What the pieces read come to: the date they name, the shift they ask
// for, and the zone they were written in.
function __strtotime_gathered($held) {
    $when = __days_of_date($held["year"], $held["mon"], $held["mday"]) * 86400
          + $held["hours"] * 3600 + $held["minutes"] * 60 + $held["seconds"] - $held["offset"];
    $shift = $held["shift"];
    if ($shift[0] !== 0 || $shift[1] !== 0) {
        $months = $held["year"] * 12 + ($held["mon"] - 1) + $shift[0] * 12 + $shift[1];
        $year = __floor_div($months, 12);
        $mon = __floor_rem($months, 12) + 1;
        $mday = $held["mday"];
        $most = __days_in_month($mon, $year);
        // PHP lets a day past the end of the month run on into the next.
        $over = $mday > $most ? $mday - $most : 0;
        if ($over > 0) { $mday = $most; }
        $when = __days_of_date($year, $mon, $mday) * 86400
              + $held["hours"] * 3600 + $held["minutes"] * 60 + $held["seconds"] - $held["offset"]
              + $over * 86400;
    }
    $when = $when + $shift[2];
    if ($held["weekday"] !== null) {
        $wday = __floor_rem(__floor_div($when, 86400) + 4, 7);
        $onward = __floor_rem($held["weekday"] - $wday, 7);
        $which = $held["which"];
        if ($which > 0) { $onward = $onward === 0 ? 7 * $which : $onward + 7 * ($which - 1); }
        else if ($which < 0) { $onward = $onward === 0 ? 7 * $which : $onward - 7 - 7 * (-$which - 1); }
        $when = $when + $onward * 86400;
    }
    return $when;
}

// One piece of what was written, whatever it turns out to be. The place
// after it is given back, or false where it says nothing at all.
function __strtotime_piece($said, $at, &$held) {
    $rest = substr($said, $at);
    $m = array();
    // A date written the way the standard has it, with the time beside
    // it and perhaps the zone after that.
    if (preg_match('/^(\d{4})-(\d{2})-(\d{2})([t ](\d{1,2}):(\d{2})(:(\d{2}))?)?/', $rest, $m)) {
        $held["year"] = (int) $m[1];
        $held["mon"] = (int) $m[2];
        $held["mday"] = (int) $m[3];
        if (isset($m[4]) && $m[4] !== "") {
            $held["hours"] = (int) $m[5];
            $held["minutes"] = (int) $m[6];
            $held["seconds"] = isset($m[8]) && $m[8] !== "" ? (int) $m[8] : 0;
        } else {
            $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
        }
        return $at + strlen($m[0]);
    }
    if (preg_match('/^(\d{4})\/(\d{1,2})\/(\d{1,2})/', $rest, $m)) {
        $held["year"] = (int) $m[1];
        $held["mon"] = (int) $m[2];
        $held["mday"] = (int) $m[3];
        $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
        return $at + strlen($m[0]);
    }
    // A date written with the month first, as it is written with slashes
    // between, and one written with the day first, as it is with dashes
    // or dots.
    if (preg_match('/^(\d{1,2})\/(\d{1,2})\/(\d{4})/', $rest, $m)) {
        $held["mon"] = (int) $m[1];
        $held["mday"] = (int) $m[2];
        $held["year"] = (int) $m[3];
        $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
        return $at + strlen($m[0]);
    }
    if (preg_match('/^(\d{1,2})[-.](\d{1,2})[-.](\d{4})/', $rest, $m)) {
        $held["mday"] = (int) $m[1];
        $held["mon"] = (int) $m[2];
        $held["year"] = (int) $m[3];
        $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
        return $at + strlen($m[0]);
    }
    // A day, a month by name and a year, in either order.
    if (preg_match('/^(\d{1,2})(st|nd|rd|th)? ([a-z]{3,9})\.?( (\d{4}))?/', $rest, $m)) {
        $mon = __month_numbered($m[3]);
        if ($mon > 0) {
            $held["mday"] = (int) $m[1];
            $held["mon"] = $mon;
            if (isset($m[5]) && $m[5] !== "") { $held["year"] = (int) $m[5]; }
            $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
            return $at + strlen($m[0]);
        }
    }
    if (preg_match('/^([a-z]{3,9})\.? (\d{1,2})(st|nd|rd|th)?,?( (\d{4}))?/', $rest, $m)) {
        $mon = __month_numbered($m[1]);
        if ($mon > 0) {
            $held["mon"] = $mon;
            $held["mday"] = (int) $m[2];
            if (isset($m[5]) && $m[5] !== "") { $held["year"] = (int) $m[5]; }
            $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
            return $at + strlen($m[0]);
        }
    }
    // A time of day, with the hours counted to twelve where it says so.
    if (preg_match('/^(\d{1,2}):(\d{2})(:(\d{2}))?( ?(am|pm))?/', $rest, $m)) {
        $hours = (int) $m[1];
        if (isset($m[6]) && $m[6] === "pm" && $hours < 12) { $hours = $hours + 12; }
        if (isset($m[6]) && $m[6] === "am" && $hours === 12) { $hours = 0; }
        $held["hours"] = $hours;
        $held["minutes"] = (int) $m[2];
        $held["seconds"] = isset($m[4]) && $m[4] !== "" ? (int) $m[4] : 0;
        return $at + strlen($m[0]);
    }
    if (preg_match('/^(\d{1,2}) ?(am|pm)/', $rest, $m)) {
        $hours = (int) $m[1];
        if ($m[2] === "pm" && $hours < 12) { $hours = $hours + 12; }
        if ($m[2] === "am" && $hours === 12) { $hours = 0; }
        $held["hours"] = $hours;
        $held["minutes"] = 0;
        $held["seconds"] = 0;
        return $at + strlen($m[0]);
    }
    // A zone written as an offset from the meridian, or named.
    if (preg_match('/^([+-]\d{2}):?(\d{2})/', $rest, $m)) {
        $seconds = ((int) substr($m[1], 1)) * 3600 + ((int) $m[2]) * 60;
        $held["offset"] = $m[1][0] === "-" ? -$seconds : $seconds;
        return $at + strlen($m[0]);
    }
    if (preg_match('/^(utc|gmt|z)\b/', $rest, $m)) { $held["offset"] = 0; return $at + strlen($m[0]); }
    // Words for a day whole, and words that shift the time.
    if (preg_match('/^(now)\b/', $rest, $m)) { return $at + strlen($m[0]); }
    if (preg_match('/^(today|midnight)\b/', $rest, $m)) {
        $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
        return $at + strlen($m[0]);
    }
    if (preg_match('/^noon\b/', $rest, $m)) {
        $held["hours"] = 12; $held["minutes"] = 0; $held["seconds"] = 0;
        return $at + strlen($m[0]);
    }
    if (preg_match('/^(tomorrow|yesterday)\b/', $rest, $m)) {
        $by = $m[1] === "tomorrow" ? 86400 : -86400;
        $shift = $held["shift"];
        $shift[2] = $shift[2] + $by;
        $held["shift"] = $shift;
        $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
        return $at + strlen($m[0]);
    }
    if (preg_match('/^([+-]?\d+) *(second|sec|minute|min|hour|day|week|fortnight|month|year)s?( +ago)?\b/', $rest, $m)) {
        $by = (int) $m[1];
        if (isset($m[3]) && trim($m[3]) === "ago") { $by = -$by; }
        __strtotime_shift($held, $m[2], $by);
        return $at + strlen($m[0]);
    }
    if (preg_match('/^(next|last|this|previous) +([a-z]+)\b/', $rest, $m)) {
        $which = $m[1] === "next" ? 1 : ($m[1] === "this" ? 0 : -1);
        $wday = __weekday_numbered($m[2]);
        if ($wday >= 0) {
            $held["weekday"] = $wday;
            $held["which"] = $which;
            $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
            return $at + strlen($m[0]);
        }
        __strtotime_shift($held, $m[2], $which === 0 ? 0 : $which);
        return $at + strlen($m[0]);
    }
    if (preg_match('/^([a-z]+)\b/', $rest, $m)) {
        $wday = __weekday_numbered($m[1]);
        if ($wday >= 0) {
            $held["weekday"] = $wday;
            $held["which"] = 0;
            $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
            return $at + strlen($m[0]);
        }
        $mon = __month_numbered($m[1]);
        if ($mon > 0) {
            $held["mon"] = $mon;
            $held["hours"] = 0; $held["minutes"] = 0; $held["seconds"] = 0;
            return $at + strlen($m[0]);
        }
    }
    return false;
}
function __strtotime_shift(&$held, $unit, $by) {
    $shift = $held["shift"];
    if ($unit === "year") { $shift[0] = $shift[0] + $by; }
    else if ($unit === "month") { $shift[1] = $shift[1] + $by; }
    else if ($unit === "week") { $shift[2] = $shift[2] + $by * 604800; }
    else if ($unit === "fortnight") { $shift[2] = $shift[2] + $by * 1209600; }
    else if ($unit === "day") { $shift[2] = $shift[2] + $by * 86400; }
    else if ($unit === "hour") { $shift[2] = $shift[2] + $by * 3600; }
    else if ($unit === "minute" || $unit === "min") { $shift[2] = $shift[2] + $by * 60; }
    else { $shift[2] = $shift[2] + $by; }
    $held["shift"] = $shift;
}
function __month_numbered($word) {
    $names = array("january","february","march","april","may","june","july","august","september","october","november","december");
    $at = 0;
    while ($at < 12) {
        if ($names[$at] === $word || substr($names[$at], 0, 3) === $word) { return $at + 1; }
        $at = $at + 1;
    }
    if ($word === "sept") { return 9; }
    return 0;
}
function __weekday_numbered($word) {
    $names = array("sunday","monday","tuesday","wednesday","thursday","friday","saturday");
    $at = 0;
    while ($at < 7) {
        if ($names[$at] === $word || substr($names[$at], 0, 3) === $word) { return $at; }
        $at = $at + 1;
    }
    return -1;
}

// ---- the classes a program holds a time in ----

class DateTimeZone {
    private $named;
    function __construct($timezone = "UTC") { $this->named = $timezone; }
    function getName() { return $this->named; }
    function getOffset($datetime = null) { return __zone_offset($this->named); }
}

interface DateTimeInterface {}

// A moment in time, held as the seconds since the start of the year the
// clock counts from, and the zone it is spoken of in.
class DateTime implements DateTimeInterface {
    public $when = 0;
    public $zone = "UTC";
    function __construct($datetime = "now", $timezone = null) {
        $this->zone = $timezone === null ? __zone_named(null) : $timezone->getName();
        $held = strtotime($datetime === "" ? "now" : $datetime);
        if ($held === false) {
            throw new Exception('DateTime::__construct(): Failed to parse time string (' . $datetime . ')');
        }
        $this->when = $held;
    }
    function getTimestamp() { return $this->when; }
    function setTimestamp($timestamp) { $this->when = $timestamp; return $this; }
    function getTimezone() { return new DateTimeZone($this->zone); }
    function setTimezone($timezone) { $this->zone = $timezone->getName(); return $this; }
    function getOffset() { return __zone_offset($this->zone); }
    function format($format) { return date($format, $this->when + __zone_offset($this->zone)); }
    function modify($modifier) {
        $held = strtotime($modifier, $this->when);
        if ($held === false) { return false; }
        $this->when = $held;
        return $this;
    }
    function setDate($year, $month, $day) {
        $m = __moment($this->when);
        $this->when = __days_of_date($year, $month, $day) * 86400 + $m["hours"] * 3600 + $m["minutes"] * 60 + $m["seconds"];
        return $this;
    }
    function setTime($hour, $minute, $second = 0, $microsecond = 0) {
        $days = __floor_div($this->when, 86400);
        $this->when = $days * 86400 + $hour * 3600 + $minute * 60 + $second;
        return $this;
    }
    function add($interval) { $this->when = __interval_moved($this->when, $interval, 1); return $this; }
    function sub($interval) { $this->when = __interval_moved($this->when, $interval, -1); return $this; }
    function diff($targetObject, $absolute = false) { return __interval_between($this->when, $targetObject->getTimestamp(), $absolute); }
}

// The same, save that nothing about it is ever written over: every
// change hands back a moment of its own.
class DateTimeImmutable implements DateTimeInterface {
    public $when = 0;
    public $zone = "UTC";
    function __construct($datetime = "now", $timezone = null) {
        $held = new DateTime($datetime, $timezone);
        $this->when = $held->getTimestamp();
        $this->zone = $held->zone;
    }
    function getTimestamp() { return $this->when; }
    function getTimezone() { return new DateTimeZone($this->zone); }
    function getOffset() { return __zone_offset($this->zone); }
    function format($format) { return date($format, $this->when + __zone_offset($this->zone)); }
    function setTimestamp($timestamp) { return __another_moment($timestamp, $this->zone); }
    function setTimezone($timezone) { return __another_moment($this->when, $timezone->getName()); }
    function modify($modifier) {
        $held = strtotime($modifier, $this->when);
        if ($held === false) { return false; }
        return __another_moment($held, $this->zone);
    }
    function setDate($year, $month, $day) {
        $m = __moment($this->when);
        return __another_moment(__days_of_date($year, $month, $day) * 86400 + $m["hours"] * 3600 + $m["minutes"] * 60 + $m["seconds"], $this->zone);
    }
    function setTime($hour, $minute, $second = 0, $microsecond = 0) {
        $days = __floor_div($this->when, 86400);
        return __another_moment($days * 86400 + $hour * 3600 + $minute * 60 + $second, $this->zone);
    }
    function add($interval) { return __another_moment(__interval_moved($this->when, $interval, 1), $this->zone); }
    function sub($interval) { return __another_moment(__interval_moved($this->when, $interval, -1), $this->zone); }
    function diff($targetObject, $absolute = false) { return __interval_between($this->when, $targetObject->getTimestamp(), $absolute); }
}
function __another_moment($when, $zone) {
    $held = new DateTimeImmutable("@0");
    $held->when = $when;
    $held->zone = $zone;
    return $held;
}

// A stretch of time: so many years, months, days, hours, minutes and
// seconds, and which way round it runs.
class DateInterval {
    public $y = 0;
    public $m = 0;
    public $d = 0;
    public $h = 0;
    public $i = 0;
    public $s = 0;
    public $f = 0;
    public $invert = 0;
    public $days = false;
    function __construct($duration = "P0D") {
        $held = array();
        if (!preg_match('/^P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?$/', $duration, $held)) {
            throw new Exception('DateInterval::__construct(): Unknown or bad format (' . $duration . ')');
        }
        $this->y = isset($held[1]) && $held[1] !== "" ? (int) $held[1] : 0;
        $this->m = isset($held[2]) && $held[2] !== "" ? (int) $held[2] : 0;
        $weeks = isset($held[3]) && $held[3] !== "" ? (int) $held[3] : 0;
        $this->d = (isset($held[4]) && $held[4] !== "" ? (int) $held[4] : 0) + $weeks * 7;
        $this->h = isset($held[5]) && $held[5] !== "" ? (int) $held[5] : 0;
        $this->i = isset($held[6]) && $held[6] !== "" ? (int) $held[6] : 0;
        $this->s = isset($held[7]) && $held[7] !== "" ? (int) $held[7] : 0;
    }
    function format($format) {
        $out = "";
        $at = 0;
        while ($at < strlen($format)) {
            $c = $format[$at];
            if ($c !== "%") { $out = $out . $c; $at = $at + 1; continue; }
            $at = $at + 1;
            if ($at >= strlen($format)) { break; }
            $next = $format[$at];
            $at = $at + 1;
            if ($next === "%") { $out = $out . "%"; }
            else if ($next === "y") { $out = $out . $this->y; }
            else if ($next === "Y") { $out = $out . str_pad((string) $this->y, 4, "0", STR_PAD_LEFT); }
            else if ($next === "m") { $out = $out . $this->m; }
            else if ($next === "M") { $out = $out . str_pad((string) $this->m, 2, "0", STR_PAD_LEFT); }
            else if ($next === "d") { $out = $out . $this->d; }
            else if ($next === "D") { $out = $out . str_pad((string) $this->d, 2, "0", STR_PAD_LEFT); }
            else if ($next === "h") { $out = $out . $this->h; }
            else if ($next === "H") { $out = $out . str_pad((string) $this->h, 2, "0", STR_PAD_LEFT); }
            else if ($next === "i") { $out = $out . $this->i; }
            else if ($next === "I") { $out = $out . str_pad((string) $this->i, 2, "0", STR_PAD_LEFT); }
            else if ($next === "s") { $out = $out . $this->s; }
            else if ($next === "S") { $out = $out . str_pad((string) $this->s, 2, "0", STR_PAD_LEFT); }
            else if ($next === "a") { $out = $out . ($this->days === false ? "(unknown)" : $this->days); }
            else if ($next === "R") { $out = $out . ($this->invert ? "-" : "+"); }
            else if ($next === "r") { $out = $out . ($this->invert ? "-" : ""); }
            else { $out = $out . $next; }
        }
        return $out;
    }
}

// A moment moved by a stretch of time, the months counted as months and
// the rest as seconds.
function __interval_moved($when, $interval, $way) {
    if ($interval->invert) { $way = -$way; }
    $m = __moment($when);
    $months = $m["year"] * 12 + ($m["mon"] - 1) + $way * ($interval->y * 12 + $interval->m);
    $year = __floor_div($months, 12);
    $mon = __floor_rem($months, 12) + 1;
    $mday = $m["mday"];
    $most = __days_in_month($mon, $year);
    $over = $mday > $most ? $mday - $most : 0;
    if ($over > 0) { $mday = $most; }
    $held = __days_of_date($year, $mon, $mday) * 86400 + $m["hours"] * 3600 + $m["minutes"] * 60 + $m["seconds"];
    $held = $held + $over * 86400;
    return $held + $way * ($interval->d * 86400 + $interval->h * 3600 + $interval->i * 60 + $interval->s);
}

// The stretch of time between two moments, counted the way a calendar
// counts it: whole years, then whole months, then the rest.
function __interval_between($from, $to, $absolute) {
    $held = new DateInterval("P0D");
    $backward = $to < $from;
    $early = $backward ? $to : $from;
    $late = $backward ? $from : $to;
    $held->days = intdiv($late - $early, 86400);
    $a = __moment($early);
    $b = __moment($late);
    $years = $b["year"] - $a["year"];
    $months = $b["mon"] - $a["mon"];
    $days = $b["mday"] - $a["mday"];
    $hours = $b["hours"] - $a["hours"];
    $minutes = $b["minutes"] - $a["minutes"];
    $seconds = $b["seconds"] - $a["seconds"];
    if ($seconds < 0) { $seconds = $seconds + 60; $minutes = $minutes - 1; }
    if ($minutes < 0) { $minutes = $minutes + 60; $hours = $hours - 1; }
    if ($hours < 0) { $hours = $hours + 24; $days = $days - 1; }
    if ($days < 0) {
        $mon = $b["mon"] - 1;
        $year = $b["year"];
        if ($mon < 1) { $mon = 12; $year = $year - 1; }
        $days = $days + __days_in_month($mon, $year);
        $months = $months - 1;
    }
    if ($months < 0) { $months = $months + 12; $years = $years - 1; }
    $held->y = $years;
    $held->m = $months;
    $held->d = $days;
    $held->h = $hours;
    $held->i = $minutes;
    $held->s = $seconds;
    $held->invert = $backward && !$absolute ? 1 : 0;
    return $held;
}

// The same, written as a program that does not want the classes.
function date_create($datetime = "now", $timezone = null) {
    try { return new DateTime($datetime, $timezone); } catch (Exception $e) { return false; }
}
function date_create_immutable($datetime = "now", $timezone = null) {
    try { return new DateTimeImmutable($datetime, $timezone); } catch (Exception $e) { return false; }
}
function date_format($object, $format) { return $object->format($format); }
function date_timestamp_get($object) { return $object->getTimestamp(); }
function date_diff($baseObject, $targetObject, $absolute = false) { return $baseObject->diff($targetObject, $absolute); }
function date_add($object, $interval) { return $object->add($interval); }
function date_sub($object, $interval) { return $object->sub($interval); }
function date_interval_format($object, $format) { return $object->format($format); }
function date_timezone_get($object) { return $object->getTimezone(); }
function timezone_open($timezone) { return new DateTimeZone($timezone); }
