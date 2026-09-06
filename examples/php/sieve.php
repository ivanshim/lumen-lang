<?php
// Ported from examples/lumen/sieve.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function substring($s, $from_start, $to_end) {
    $index = $from_start;
    $out = "";
    while ($index < $to_end) {
        $out = $out . $s[$index];
        $index = $index + 1;
    }
    return $out;
}

function substring_start($s, $to_here) {
    return substring($s, 0, $to_here);
}

function primes_up_to($limit) {
    $sieve = [];
    $i = 0;
    while ($i <= $limit) {
        array_push($sieve, true);
        $i = $i + 1;
    }
    $sieve[0] = false;
    $sieve[1] = false;
    $p = 2;
    while ($p * $p <= $limit) {
        if ($sieve[$p]) {
            $k = $p * $p;
            while ($k <= $limit) {
                $sieve[$k] = false;
                $k = $k + $p;
            }
        }
        $p = $p + 1;
    }
    $primes = [];
    $i = 2;
    while ($i <= $limit) {
        if ($sieve[$i]) {
            array_push($primes, $i);
        }
        $i = $i + 1;
    }
    return $primes;
}

$result = primes_up_to(10000);
$result_string = strval($result);
print(substring_start($result_string, 100) . "\n");
