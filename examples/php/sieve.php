<?php
// Ported from examples/lumen/sieve.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$result = primes_up_to(10000);
$result_string = strval($result);
print(substring_start($result_string, 100) . "\n");
