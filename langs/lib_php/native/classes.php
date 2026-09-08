// What PHP lets a program ask about its own classes and routines, and
// the numbers it works out that the kernel has no word of its own for.
// Written in PHP because it is PHP's and not the kernel's.
// Hand-written; scripts/port_examples.py leaves native/ alone.

// Whether a class of that name has been bound. Names are told apart
// however they are written, as a class is.
function class_exists($class, $autoload = true) {
    $wanted = strtolower($class);
    foreach (__classes_bound() as $bound) {
        if (strtolower($bound) === $wanted) { return true; }
    }
    return false;
}
function interface_exists($interface, $autoload = true) { return class_exists($interface); }
function enum_exists($enum, $autoload = true) { return false; }

// Whether a class, or the class of a thing, has a method of that name.
function method_exists($object_or_class, $method) {
    $wanted = strtolower($method);
    foreach (__class_methods($object_or_class) as $called) {
        if (strtolower($called) === $wanted) { return true; }
    }
    return false;
}
function get_class_methods($object_or_class) { return __class_methods($object_or_class); }

// Whether a class, or a thing, has a property of that name. A thing may
// have been given one that its class never named.
function property_exists($object_or_class, $property) {
    foreach (__class_properties($object_or_class) as $named) {
        if ($named === $property) { return true; }
    }
    if (is_object($object_or_class)) {
        foreach (get_object_vars($object_or_class) as $named => $value) {
            if ($named === $property) { return true; }
        }
    }
    return false;
}

// Whether a thing is of a class, or of one built on it. A name may
// stand for the thing where the caller allows it, and then only the
// classes it is built on are walked, since a name carries nothing else.
function is_a($object_or_class, $class, $allow_string = false) {
    if (is_object($object_or_class)) { return $object_or_class instanceof $class; }
    if (!is_string($object_or_class) || !$allow_string) { return false; }
    return __built_on($object_or_class, $class, true);
}
function is_subclass_of($object_or_class, $class, $allow_string = true) {
    if (is_object($object_or_class)) {
        return ($object_or_class instanceof $class) && strtolower(get_class($object_or_class)) !== strtolower($class);
    }
    if (!is_string($object_or_class) || !$allow_string) { return false; }
    return __built_on($object_or_class, $class, false);
}
function __built_on($named, $class, $itself) {
    $wanted = strtolower($class);
    $here = $itself ? $named : get_parent_class($named);
    while ($here !== false && $here !== null) {
        if (strtolower($here) === $wanted) { return true; }
        $here = get_parent_class($here);
    }
    return false;
}

// Everything a walk hands out, as an array.
function iterator_to_array($iterator, $preserve_keys = true) {
    $out = array();
    foreach ($iterator as $k => $v) {
        if ($preserve_keys) { $out[$k] = $v; } else { $out[] = $v; }
    }
    return $out;
}
function iterator_count($iterator) {
    $held = 0;
    foreach ($iterator as $v) { $held = $held + 1; }
    return $held;
}

// Whether a value stands where a routine stands: a name that spells one,
// a routine itself, or a thing paired with a method's name.
function is_callable($value, $syntax_only = false, &$callable_name = null) {
    if (is_string($value)) {
        $callable_name = $value;
        return function_exists($value) || strpos($value, '::') !== false;
    }
    if (is_array($value) && count($value) === 2) {
        $of = is_object($value[0]) ? get_class($value[0]) : $value[0];
        $callable_name = $of . '::' . $value[1];
        return method_exists($value[0], $value[1]);
    }
    $callable_name = 'Closure::__invoke';
    return __is_routine($value);
}
function __is_routine($value) {
    return !is_string($value) && !is_array($value) && !is_object($value)
        && !is_int($value) && !is_float($value) && !is_bool($value) && $value !== null;
}

// ---- numbers the kernel has no word of its own for ----

// The greatest whole number that is no greater, and the least that is no
// less, both given back as reals, which is what PHP hands back. A number
// too wide for a whole number to hold is already whole.
function floor($num) {
    $x = (float) $num;
    if ($x >= 9223372036854775808.0 || $x <= -9223372036854775808.0) { return $x; }
    $cut = (float) (int) $x;
    return $x < 0 && $cut != $x ? $cut - 1 : $cut;
}
function ceil($num) {
    $x = (float) $num;
    if ($x >= 9223372036854775808.0 || $x <= -9223372036854775808.0) { return $x; }
    $cut = (float) (int) $x;
    $up = $x > 0 && $cut != $x ? $cut + 1 : $cut;
    // A number below nought that comes up to nought comes up to the
    // nought below nought, which is a real of its own.
    return $up == 0.0 && $x < 0 ? -0.0 : $up;
}

// How many tens a number stands at: the power of ten of its first
// figure, counted rather than worked out, since the kernel has no word
// for the logarithm.
function __log10_floor($x) {
    $v = $x < 0 ? -$x : $x;
    $n = 0;
    if ($v >= 1.0) {
        while ($v >= 10.0) { $v = $v / 10.0; $n = $n + 1; }
    } else {
        while ($v < 1.0) { $v = $v * 10.0; $n = $n - 1; }
    }
    return $n;
}

// The nearest whole number, the half going away from nought.
function __half_away($v) {
    if ($v >= 9223372036854775808.0 || $v <= -9223372036854775808.0) { return $v; }
    $cut = (float) (int) $v;
    $over = $v - $cut;
    if ($over >= 0.5) { return $cut + 1; }
    if ($over <= -0.5) { return $cut - 1; }
    return $cut;
}

// A number brought to so many figures after the point, the half going
// away from nought, which is how PHP rounds. Figures below nought round
// to the tens, the hundreds and so on.
function round($num, $precision = 0, $mode = 1) {
    $x = (float) $num;
    if ($x == 0.0) { return $x; }
    $places = (int) $precision;
    // A real of the width seldom stands exactly where the figures a
    // reader wrote it with stand: 0.285 is a shade under. So it is
    // first brought to as many figures as the width can tell apart,
    // which puts it back where it was written, and only then to the
    // figures asked for. That is what PHP does.
    $enough = 14 - __log10_floor($x);
    if ($enough > $places && $enough - 15 < $places) {
        $lifted = __half_away($x * (10 ** $enough)) / (10 ** ($enough - $places));
        $whole = __half_away($lifted);
    } else {
        $whole = __half_away($x * (10 ** $places));
    }
    $held = $whole / (10 ** $places);
    // A number below nought that rounds to nought rounds to the nought
    // below nought, which keeps its minus.
    return $held == 0.0 && $x < 0 ? -0.0 : $held;
}

// What is left over when one number is divided by another as reals, the
// sign following the first.
function fmod($num1, $num2) {
    $x = (float) $num1;
    $y = (float) $num2;
    if ($y == 0.0) { return 0.0; }
    $times = $x / $y;
    $whole = $times < 0 ? ceil($times) : floor($times);
    return $x - $whole * $y;
}

// One number raised to another. A whole number raised to a whole number
// that is not below nought stays whole, as PHP has it.
function pow($num, $exponent) { return $num ** $exponent; }
