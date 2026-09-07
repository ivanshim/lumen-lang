# Ported from examples/lumen/sieve.lm by scripts/port_examples.py; edit the Lumen original, not this file.
result = primes_up_to(10000)
result_string = result.to_s
puts(substring_start(result_string, 100))
