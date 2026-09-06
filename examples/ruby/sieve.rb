# Ported from examples/lumen/sieve.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def substring(s, from_start, to_end)
    index = from_start
    out = ""
    while index < to_end do
        out = out + s[index]
        index = index + 1
    end
    return out
end

def substring_start(s, to_here)
    return substring(s, 0, to_here)
end

def primes_up_to(limit)
    sieve = []
    i = 0
    while i <= limit do
        sieve.push(true)
        i = i + 1
    end
    sieve[0] = false
    sieve[1] = false
    p = 2
    while p * p <= limit do
        if sieve[p] then
            k = p * p
            while k <= limit do
                sieve[k] = false
                k = k + p
            end
        end
        p = p + 1
    end
    primes = []
    i = 2
    while i <= limit do
        if sieve[i] then
            primes.push(i)
        end
        i = i + 1
    end
    return primes
end

result = primes_up_to(10000)
result_string = result.to_s
puts(substring_start(result_string, 100))
