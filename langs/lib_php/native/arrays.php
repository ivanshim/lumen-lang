// What PHP does with arrays beyond the kernel's own words: mapping,
// sifting, gathering and the rest, written in PHP because they are
// PHP's and not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// Which of the three an array is sifted by.
define("ARRAY_FILTER_USE_KEY", 2);
define("ARRAY_FILTER_USE_BOTH", 1);

// A run of an array. A place below nought counts back from the end, and
// so does a length below nought, which says where to stop rather than
// how many to take. Keys that are text are always kept; keys that are
// numbers are counted afresh unless the caller asks otherwise.
function array_slice($array, $offset, $length = null, $preserve_keys = false) {
    $keys = array();
    $items = array();
    foreach ($array as $k => $v) { $keys[] = $k; $items[] = $v; }
    $held = count($items);
    $from = $offset < 0 ? $held + $offset : $offset;
    if ($from < 0) { $from = 0; }
    if ($from > $held) { $from = $held; }
    if ($length === null) { $upto = $held; }
    else if ($length < 0) { $upto = $held + $length; }
    else { $upto = $from + $length; }
    if ($upto > $held) { $upto = $held; }
    $out = array();
    $at = $from;
    while ($at < $upto) {
        if ($preserve_keys || is_string($keys[$at])) { $out[$keys[$at]] = $items[$at]; }
        else { $out[] = $items[$at]; }
        $at = $at + 1;
    }
    return $out;
}

// The properties of a thing that can be reached from where the asking
// is done, by name.
function get_object_vars($object) {
    $out = array();
    foreach ($object as $k => $v) { $out[$k] = $v; }
    return $out;
}

// Each item put through a routine. Given one array the keys are kept as
// they stand; given more, the arrays are walked side by side, the short
// ones filled out with nothing, and the answers counted from nought.
// Nothing standing where the routine goes gathers the items together.
function array_map($callback, $array) {
    $rest = array_slice(func_get_args(), 2);
    if (count($rest) === 0) {
        if ($callback === null) { return $array; }
        $out = array();
        foreach ($array as $k => $v) { $out[$k] = $callback($v); }
        return $out;
    }
    $lists = array_merge(array($array), $rest);
    $longest = 0;
    foreach ($lists as $list) { if (count($list) > $longest) { $longest = count($list); } }
    $out = array();
    $at = 0;
    while ($at < $longest) {
        $row = array();
        foreach ($lists as $list) {
            $values = array_values($list);
            $row[] = $at < count($values) ? $values[$at] : null;
        }
        $out[] = $callback === null ? $row : call_user_func_array($callback, $row);
        $at = $at + 1;
    }
    return $out;
}

// The items a routine says yes to, keeping the keys they stood under.
// The routine is handed the value, or the key, or both, as the mode
// says; no routine at all keeps whatever is true.
function array_filter($array, $callback = null, $mode = 0) {
    $out = array();
    foreach ($array as $k => $v) {
        if ($callback === null) { $keep = (bool) $v; }
        else if ($mode === ARRAY_FILTER_USE_KEY) { $keep = (bool) $callback($k); }
        else if ($mode === ARRAY_FILTER_USE_BOTH) { $keep = (bool) $callback($v, $k); }
        else { $keep = (bool) $callback($v); }
        if ($keep) { $out[$k] = $v; }
    }
    return $out;
}

// The items brought together one at a time, each answer carried into
// the next call.
function array_reduce($array, $callback, $initial = null) {
    $carried = $initial;
    foreach ($array as $v) { $carried = $callback($carried, $v); }
    return $carried;
}

// So many of one value, counted from a first key. A first key below
// nought counts on from there, as PHP has done since 8.0.
function array_fill($start_index, $count, $value) {
    if ($count < 0) { throw new ValueError('array_fill(): Argument #2 ($count) must be greater than or equal to 0'); }
    $out = array();
    $at = 0;
    while ($at < $count) { $out[$start_index + $at] = $value; $at = $at + 1; }
    return $out;
}

// One value under every one of the given keys.
function array_fill_keys($keys, $value) {
    $out = array();
    foreach ($keys as $k) { $out[$k] = $value; }
    return $out;
}

// One array's items under another's keys. The two must be of a length.
function array_combine($keys, $values) {
    if (count($keys) !== count($values)) {
        throw new ValueError('array_combine(): Argument #1 ($keys) and argument #2 ($values) must have the same number of elements');
    }
    $ks = array_values($keys);
    $vs = array_values($values);
    $out = array();
    $at = 0;
    while ($at < count($ks)) { $out[$ks[$at]] = $vs[$at]; $at = $at + 1; }
    return $out;
}

// An array brought up to a length with one value, at the end where the
// length is above nought and at the front where it is below.
function array_pad($array, $length, $value) {
    $want = $length < 0 ? -$length : $length;
    $items = array_values($array);
    if ($want <= count($items)) { return $items; }
    $filling = array();
    $at = count($items);
    while ($at < $want) { $filling[] = $value; $at = $at + 1; }
    return $length < 0 ? array_merge($filling, $items) : array_merge($items, $filling);
}

// An array cut into runs of a length. The keys are dropped unless the
// caller asks for them.
function array_chunk($array, $length, $preserve_keys = false) {
    if ($length < 1) { throw new ValueError('array_chunk(): Argument #2 ($length) must be greater than 0'); }
    $out = array();
    $run = array();
    $held = 0;
    foreach ($array as $k => $v) {
        if ($preserve_keys) { $run[$k] = $v; } else { $run[] = $v; }
        $held = $held + 1;
        if ($held === $length) { $out[] = $run; $run = array(); $held = 0; }
    }
    if ($held > 0) { $out[] = $run; }
    return $out;
}

// Everything added up, and everything multiplied together. Nothing at
// all adds up to nought and multiplies to one.
function array_sum($array) {
    $total = 0;
    foreach ($array as $v) { $total = $total + $v; }
    return $total;
}
function array_product($array) {
    $total = 1;
    foreach ($array as $v) { $total = $total * $v; }
    return $total;
}

// The first key an array holds, and the last. Nothing where it holds
// nothing at all.
function array_key_first($array) {
    foreach ($array as $k => $v) { return $k; }
    return null;
}
function array_key_last($array) {
    $last = null;
    foreach ($array as $k => $v) { $last = $k; }
    return $last;
}

// Each value once, the first standing for the rest. Values are told
// apart as text unless the caller asks for them to be told apart as
// they stand.
function array_unique($array, $flags = SORT_STRING) {
    $out = array();
    $seen = array();
    foreach ($array as $k => $v) {
        $mark = $flags === SORT_REGULAR ? $v : (string) $v;
        $already = false;
        foreach ($seen as $s) {
            if ($flags === SORT_REGULAR ? ($s == $mark) : ($s === $mark)) { $already = true; }
        }
        if (!$already) { $seen[] = $mark; $out[$k] = $v; }
    }
    return $out;
}

// What the first array holds and the others do not, and what they all
// hold. Values are told apart as text, keys are kept as they stand.
function array_diff($array) {
    $others = array_slice(func_get_args(), 1);
    $out = array();
    foreach ($array as $k => $v) {
        $found = false;
        foreach ($others as $other) {
            foreach ($other as $w) { if ((string) $w === (string) $v) { $found = true; } }
        }
        if (!$found) { $out[$k] = $v; }
    }
    return $out;
}
function array_intersect($array) {
    $others = array_slice(func_get_args(), 1);
    $out = array();
    foreach ($array as $k => $v) {
        $in_all = true;
        foreach ($others as $other) {
            $here = false;
            foreach ($other as $w) { if ((string) $w === (string) $v) { $here = true; } }
            if (!$here) { $in_all = false; }
        }
        if ($in_all) { $out[$k] = $v; }
    }
    return $out;
}

// The keys of one array against the keys of the others, the values kept
// as the first array has them.
function array_diff_key($array) {
    $others = array_slice(func_get_args(), 1);
    $out = array();
    foreach ($array as $k => $v) {
        $found = false;
        foreach ($others as $other) { if (array_key_exists($k, $other)) { $found = true; } }
        if (!$found) { $out[$k] = $v; }
    }
    return $out;
}
function array_intersect_key($array) {
    $others = array_slice(func_get_args(), 1);
    $out = array();
    foreach ($array as $k => $v) {
        $in_all = true;
        foreach ($others as $other) { if (!array_key_exists($k, $other)) { $in_all = false; } }
        if ($in_all) { $out[$k] = $v; }
    }
    return $out;
}

// One column out of a table of rows, filed under another column where
// one is named. A row may be a thing as well as an array, and its
// properties stand for its columns.
function array_column($array, $column_key, $index_key = null) {
    $out = array();
    foreach ($array as $row) {
        $held = is_object($row) ? get_object_vars($row) : $row;
        if ($column_key === null) { $value = $row; }
        else if (array_key_exists($column_key, $held)) { $value = $held[$column_key]; }
        else { continue; }
        if ($index_key !== null && array_key_exists($index_key, $held)) { $out[$held[$index_key]] = $value; }
        else { $out[] = $value; }
    }
    return $out;
}

// A piece taken out of an array and another put in its place. What was
// taken is handed back; the array is left counted from nought again,
// though its string keys are kept.
function array_splice(&$array, $offset, $length = null, $replacement = array()) {
    $items = array();
    $keys = array();
    foreach ($array as $k => $v) { $keys[] = $k; $items[] = $v; }
    $held = count($items);
    $from = $offset < 0 ? $held + $offset : $offset;
    if ($from < 0) { $from = 0; }
    if ($from > $held) { $from = $held; }
    if ($length === null) { $upto = $held; }
    else if ($length < 0) { $upto = $held + $length; }
    else { $upto = $from + $length; }
    if ($upto < $from) { $upto = $from; }
    if ($upto > $held) { $upto = $held; }
    if (!is_array($replacement)) { $replacement = array($replacement); }
    $taken = array();
    $kept = array();
    $at = 0;
    while ($at < $held) {
        if ($at === $from) { foreach ($replacement as $r) { $kept[] = $r; } }
        if ($at >= $from && $at < $upto) { $taken[] = $items[$at]; }
        else if (is_string($keys[$at])) { $kept[$keys[$at]] = $items[$at]; }
        else { $kept[] = $items[$at]; }
        $at = $at + 1;
    }
    if ($from >= $held) { foreach ($replacement as $r) { $kept[] = $r; } }
    $array = $kept;
    return $taken;
}

// ---- putting an array in order ----

// A key and its value, side by side, so that an order worked out over
// the pairs can be laid back out with the keys kept or dropped.
function __pairs_of($array) {
    $pairs = array();
    foreach ($array as $k => $v) { $pairs[] = array($k, $v); }
    return $pairs;
}

// Two values weighed as one of the sort flags says: as they stand, as
// numbers, or as text.
function __weighed($a, $b, $flags) {
    if ($flags === SORT_NUMERIC) {
        $x = (float) $a;
        $y = (float) $b;
        return $x < $y ? -1 : ($x > $y ? 1 : 0);
    }
    if ($flags === SORT_STRING) {
        return strcmp((string) $a, (string) $b);
    }
    return $a <=> $b;
}

// The pairs in order. The two halves are put in order and then brought
// together, taking from the left wherever the two weigh alike, so that
// items that weigh the same stand as they stood: PHP has sorted that
// way since 8.0.
function __in_order($pairs, $how, $by, $flags) {
    $held = count($pairs);
    if ($held < 2) { return $pairs; }
    $half = (int) ($held / 2);
    $left = array();
    $right = array();
    $at = 0;
    while ($at < $held) {
        if ($at < $half) { $left[] = $pairs[$at]; } else { $right[] = $pairs[$at]; }
        $at = $at + 1;
    }
    $left = __in_order($left, $how, $by, $flags);
    $right = __in_order($right, $how, $by, $flags);
    $out = array();
    $i = 0;
    $j = 0;
    while ($i < count($left) && $j < count($right)) {
        if (__weighs_less($right[$j], $left[$i], $how, $by, $flags)) { $out[] = $right[$j]; $j = $j + 1; }
        else { $out[] = $left[$i]; $i = $i + 1; }
    }
    while ($i < count($left)) { $out[] = $left[$i]; $i = $i + 1; }
    while ($j < count($right)) { $out[] = $right[$j]; $j = $j + 1; }
    return $out;
}

// Whether one pair belongs before another: by value or by key, weighed
// by the flags or by a routine of the caller's, and turned about where
// the order asked for runs downward.
function __weighs_less($a, $b, $how, $by, $flags) {
    $x = $by === 'key' ? $a[0] : $a[1];
    $y = $by === 'key' ? $b[0] : $b[1];
    if ($how === null) { $held = __weighed($x, $y, $flags); }
    else if ($how === 'down') { $held = -__weighed($x, $y, $flags); }
    else if ($how === 'natural') { $held = strnatcmp((string) $x, (string) $y); }
    else if ($how === 'natural_fold') { $held = strnatcasecmp((string) $x, (string) $y); }
    else { $held = $how($x, $y); }
    return $held < 0;
}

// The pairs laid back out as an array, the keys kept or counted afresh.
function __laid_out($pairs, $keep_keys) {
    $out = array();
    foreach ($pairs as $p) {
        if ($keep_keys) { $out[$p[0]] = $p[1]; } else { $out[] = $p[1]; }
    }
    return $out;
}

// Values in order, the keys dropped.
function sort(&$array, $flags = SORT_REGULAR) {
    $array = __laid_out(__in_order(__pairs_of($array), null, 'value', $flags), false);
    return true;
}
function rsort(&$array, $flags = SORT_REGULAR) {
    $array = __laid_out(__in_order(__pairs_of($array), 'down', 'value', $flags), false);
    return true;
}
// Values in order, the keys kept.
function asort(&$array, $flags = SORT_REGULAR) {
    $array = __laid_out(__in_order(__pairs_of($array), null, 'value', $flags), true);
    return true;
}
function arsort(&$array, $flags = SORT_REGULAR) {
    $array = __laid_out(__in_order(__pairs_of($array), 'down', 'value', $flags), true);
    return true;
}
// Keys in order, the keys kept.
function ksort(&$array, $flags = SORT_REGULAR) {
    $array = __laid_out(__in_order(__pairs_of($array), null, 'key', $flags), true);
    return true;
}
function krsort(&$array, $flags = SORT_REGULAR) {
    $array = __laid_out(__in_order(__pairs_of($array), 'down', 'key', $flags), true);
    return true;
}
// In an order the caller works out, by value or by key.
function usort(&$array, $callback) {
    $array = __laid_out(__in_order(__pairs_of($array), $callback, 'value', SORT_REGULAR), false);
    return true;
}
function uasort(&$array, $callback) {
    $array = __laid_out(__in_order(__pairs_of($array), $callback, 'value', SORT_REGULAR), true);
    return true;
}
function uksort(&$array, $callback) {
    $array = __laid_out(__in_order(__pairs_of($array), $callback, 'key', SORT_REGULAR), true);
    return true;
}
// In the order a reader would put them, where runs of digits stand for
// the numbers they spell.
function natsort(&$array) {
    $array = __laid_out(__in_order(__pairs_of($array), 'natural', 'value', SORT_REGULAR), true);
    return true;
}
function natcasesort(&$array) {
    $array = __laid_out(__in_order(__pairs_of($array), 'natural_fold', 'value', SORT_REGULAR), true);
    return true;
}

// Two pieces of text weighed the way a reader weighs them: a run of
// digits stands for the number it spells, so `img10` comes after
// `img9`, and leading noughts count for nothing until everything else
// is alike.
function strnatcmp($a, $b) { return __naturally($a, $b, false); }
function strnatcasecmp($a, $b) { return __naturally($a, $b, true); }
function __naturally($a, $b, $fold) {
    $x = $fold ? strtolower((string) $a) : (string) $a;
    $y = $fold ? strtolower((string) $b) : (string) $b;
    $i = 0;
    $j = 0;
    while ($i < strlen($x) && $j < strlen($y)) {
        $c = $x[$i];
        $d = $y[$j];
        if (__is_digit($c) && __is_digit($d)) {
            $from = $i;
            while ($i < strlen($x) && __is_digit($x[$i])) { $i = $i + 1; }
            $to = $j;
            while ($j < strlen($y) && __is_digit($y[$j])) { $j = $j + 1; }
            $one = ltrim(substr($x, $from, $i - $from), '0');
            $two = ltrim(substr($y, $to, $j - $to), '0');
            if (strlen($one) !== strlen($two)) { return strlen($one) < strlen($two) ? -1 : 1; }
            if ($one !== $two) { return strcmp($one, $two); }
            continue;
        }
        if ($c !== $d) { return $c < $d ? -1 : 1; }
        $i = $i + 1;
        $j = $j + 1;
    }
    $left = strlen($x) - $i;
    $right = strlen($y) - $j;
    if ($left === $right) { return 0; }
    return $left < $right ? -1 : 1;
}
function __is_digit($c) { return $c >= '0' && $c <= '9'; }
