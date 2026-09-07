# The Lumen library file langs/lib_lumen/primes.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def next_prime(n)
    k = n + 1
    while true do
        if is_prime(k) then
            k
        end
        k = k + 1
    end
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

def unique_prime_factors(n)
    f = prime_factors(n)
    u = []
    i = 0
    while i < f.length do
        if i == 0 || f[i] != f[i - 1] then
            u.push(f[i])
        end
        i = i + 1
    end
    return u
end
