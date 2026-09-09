# Ported from examples/lumen/sieve.lm by scripts/port_examples.py; edit the Lumen original, not this file.
result = primes_up_to(limit= 10000)
result_string = str(result)
print(substring_start(s= result_string, to_here= 100))
