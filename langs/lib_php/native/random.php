// The numbers PHP draws at random, and what is built on them. The
// drawing is the Mersenne twister PHP itself uses, worked the same way,
// so that a run seeded alike draws alike.
// Hand-written; scripts/port_examples.py leaves native/ alone.

define("MT_RAND_MT19937", 0);
define("MT_RAND_PHP", 1);

// The twister's state: six hundred and twenty-four whole numbers of
// thirty-two bits, how many of them are still to be handed out, and
// whether it has been seeded at all.
function __mt_state($seed) {
    static $state = array();
    static $next = 624;
    static $seeded = false;
    // Seeding sets the state up and turns it over at once, and hands
    // nothing out: the first number asked for is the first of a state
    // just turned.
    if ($seed !== null) {
        $state = __mt_turned(__mt_begun($seed));
        $next = 0;
        $seeded = true;
        return 0;
    }
    if (!$seeded) {
        $state = __mt_turned(__mt_begun(((int) __clock()) & 0x7FFFFFFF));
        $next = 0;
        $seeded = true;
    }
    if ($next >= 624) {
        $state = __mt_turned($state);
        $next = 0;
    }
    $held = $state[$next];
    $next = $next + 1;
    return $held;
}

// The state a seed begins: each number worked out from the one before.
function __mt_begun($seed) {
    $state = array();
    $state[0] = $seed & 0xFFFFFFFF;
    $i = 1;
    while ($i < 624) {
        $was = $state[$i - 1];
        $state[$i] = (1812433253 * ($was ^ ($was >> 30)) + $i) & 0xFFFFFFFF;
        $i = $i + 1;
    }
    return $state;
}

// The state turned over: every number made afresh from the one after it
// and the one six hundred and twenty-four steps on.
function __mt_turned($state) {
    $out = $state;
    $i = 0;
    while ($i < 624) {
        $mixed = ($out[$i] & 0x80000000) | ($out[($i + 1) % 624] & 0x7FFFFFFF);
        $twisted = $out[($i + 397) % 624] ^ ($mixed >> 1);
        if ($mixed & 1) { $twisted = $twisted ^ 0x9908B0DF; }
        $out[$i] = $twisted & 0xFFFFFFFF;
        $i = $i + 1;
    }
    return $out;
}

// One number of thirty-two bits, the state's own tempered so that its
// bits fall evenly.
function __mt_draw() {
    $s = __mt_state(null);
    $s = $s ^ ($s >> 11);
    $s = ($s ^ (($s << 7) & 0x9D2C5680)) & 0xFFFFFFFF;
    $s = ($s ^ (($s << 15) & 0xEFC60000)) & 0xFFFFFFFF;
    return ($s ^ ($s >> 18)) & 0xFFFFFFFF;
}

// The numbers PHP draws where it wants them hard to guess come from a
// source of their own, so that drawing one does not step the twister a
// program may have seeded and be counting on.
function __other_draw() {
    static $held = 0;
    static $begun = false;
    if (!$begun) { $held = ((((int) __clock()) & 0xFFFFFFFF) | 1) & 0xFFFFFFFF; $begun = true; }
    $held = ($held ^ (($held << 13) & 0xFFFFFFFF)) & 0xFFFFFFFF;
    $held = ($held ^ ($held >> 17)) & 0xFFFFFFFF;
    $held = ($held ^ (($held << 5) & 0xFFFFFFFF)) & 0xFFFFFFFF;
    return $held;
}
function __a_draw($twister) { return $twister ? __mt_draw() : __other_draw(); }

// A number drawn from a run of them, evenly: a draw too near the top of
// the thirty-two bits to divide evenly is thrown away and drawn again.
function __mt_within($umax, $twister = true) {
    $held = __a_draw($twister);
    if ($umax === 0xFFFFFFFF) { return $held; }
    $span = $umax + 1;
    if (($span & ($span - 1)) === 0) { return $held & ($span - 1); }
    $ceiling = 0xFFFFFFFF - (0xFFFFFFFF % $span) - 1;
    while ($held > $ceiling) { $held = __a_draw($twister); }
    return $held % $span;
}
function __mt_range($min, $max, $twister = true) {
    if ($max <= $min) { return $min; }
    $span = $max - $min;
    if ($span <= 0xFFFFFFFF) { return $min + __mt_within($span, $twister); }
    // A run too wide for thirty-two bits takes two draws for one number
    // of sixty-three bits, which is as wide as a whole number goes here;
    // a draw too near the top to divide evenly is thrown away and drawn
    // again. PHP works this one over sixty-four unsigned bits, which no
    // whole number here holds, so a run wider than thirty-two bits draws
    // evenly but not the same numbers.
    $top = 9223372036854775807;
    if ($span >= $top) { return $min + __a_wide_draw($twister); }
    $span1 = $span + 1;
    $held = __a_wide_draw($twister);
    $ceiling = $top - ($top % $span1) - 1;
    while ($held > $ceiling) { $held = __a_wide_draw($twister); }
    return $min + ($held % $span1);
}
function __a_wide_draw($twister) {
    return (__a_draw($twister) & 0x7FFFFFFF) * 4294967296 + __a_draw($twister);
}

// Seeding the draw, and drawing. Both names for the older drawing stand
// for the same twister, as they have since PHP 7.1.
function mt_srand($seed = 0, $mode = MT_RAND_MT19937) {
    __mt_state((int) $seed);
    return null;
}
function srand($seed = 0, $mode = MT_RAND_MT19937) { return mt_srand($seed, $mode); }
function mt_getrandmax() { return 2147483647; }
function getrandmax() { return 2147483647; }
function mt_rand($min = null, $max = null) {
    if ($min === null) { return __mt_draw() >> 1; }
    if ($max === null) { throw new ArgumentCountError('mt_rand() expects exactly 0 or 2 arguments, 1 given'); }
    if ($min > $max) { throw new ValueError('mt_rand(): Argument #1 ($min) must be less than or equal to argument #2 ($max)'); }
    return __mt_range((int) $min, (int) $max);
}
function rand($min = null, $max = null) { return mt_rand($min, $max); }
function random_int($min, $max) {
    if ($min > $max) { throw new ValueError('random_int(): Argument #1 ($min) must be less than or equal to argument #2 ($max)'); }
    return __mt_range((int) $min, (int) $max, false);
}
function random_bytes($length) {
    $out = "";
    $at = 0;
    while ($at < $length) { $out = $out . chr(__mt_within(255, false)); $at = $at + 1; }
    return $out;
}

// An array's items put in no order, counted afresh from nought. Each
// place from the last down swaps with one drawn from those at or below
// it, which is how PHP shuffles.
function shuffle(&$array) {
    $items = array_values($array);
    $left = count($items) - 1;
    while ($left > 0) {
        $drawn = __mt_range(0, $left);
        if ($drawn !== $left) {
            $held = $items[$left];
            $items[$left] = $items[$drawn];
            $items[$drawn] = $held;
        }
        $left = $left - 1;
    }
    $array = $items;
    return true;
}
function str_shuffle($string) {
    $letters = str_split($string);
    shuffle($letters);
    return implode("", $letters);
}

// One key drawn from an array, or so many of them, kept in the order
// the array holds them.
function array_rand($array, $num = 1) {
    $keys = array_keys($array);
    $held = count($keys);
    if ($held === 0) { throw new ValueError('array_rand(): Argument #1 ($array) cannot be empty'); }
    if ($num < 1 || $num > $held) {
        throw new ValueError('array_rand(): Argument #2 ($num) must be between 1 and the number of elements in argument #1 ($array)');
    }
    if ($num === 1) { return $keys[__mt_range(0, $held - 1)]; }
    // Places are drawn until enough of them are different, and the keys
    // then come out in the order the array holds them. Asking for more
    // than half draws the ones to leave out instead, which is fewer
    // draws for the same answer.
    $leaving_out = $num > ($held >> 1);
    $want = $leaving_out ? $held - $num : $num;
    $marked = array();
    $still = $want;
    while ($still > 0) {
        $drawn = __mt_range(0, $held - 1);
        if (!isset($marked[$drawn])) { $marked[$drawn] = true; $still = $still - 1; }
    }
    $taken = array();
    $at = 0;
    while ($at < $held) {
        if (isset($marked[$at]) !== $leaving_out) { $taken[] = $keys[$at]; }
        $at = $at + 1;
    }
    return $taken;
}

// A name unlikely to have been used before: the clock, written in base
// sixteen, with more figures where the caller asks for them.
function uniqid($prefix = "", $more_entropy = false) {
    $now = (int) __clock();
    $held = $prefix . str_pad(dechex($now), 8, "0", STR_PAD_LEFT) . str_pad(dechex(__mt_within(0xFFFFF)), 5, "0", STR_PAD_LEFT);
    if ($more_entropy) { $held = $held . "." . strval(__mt_within(99999999)); }
    return $held;
}
