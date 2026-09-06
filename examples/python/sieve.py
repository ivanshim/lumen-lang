# Ported from examples/lumen/sieve.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def substring(s, from_start, to_end):
    index = from_start
    out = ""
    while index < to_end:
        out = out + s[index]
        index = index + 1
    return out

def substring_start(s, to_here):
    return substring(s, 0, to_here)

def primes_up_to(limit):
    sieve = []
    i = 0
    while i <= limit:
        sieve.append(True)
        i = i + 1
    sieve[0] = False
    sieve[1] = False
    p = 2
    while p * p <= limit:
        if sieve[p]:
            k = p * p
            while k <= limit:
                sieve[k] = False
                k = k + p
        p = p + 1
    primes = []
    i = 2
    while i <= limit:
        if sieve[i]:
            primes.append(i)
        i = i + 1
    return primes

result = primes_up_to(10000)
result_string = str(result)
print(substring_start(result_string, 100))
