# The Lumen library file langs/lib_lumen/number_theory.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def gcd(a, b)
    while b != 0 do
        t = b
        b = a % b
        a = t
    end
    return a
end

def extended_gcd(a, b)
    if b == 0 then
        return [a, 1, 0]
    else
        r = extended_gcd(b, a % b)
        g = r[0]
        x = r[2]
        y = r[1] - (a / b) * r[2]
        return [g, x, y]
    end
end

def lcm(a, b)
    return (a * b) / gcd(a, b)
end

def is_coprime(a, b)
    return gcd(a, b) == 1
end

def mod_inverse(a, m)
    r = extended_gcd(a, m)
    if r[0] != 1 then
        return raise("mod_inverse: inverse does not exist (gcd(a, m) != 1)")
    else
        return (r[1] % m + m) % m
    end
end

def mod_div(a, b, m)
    if gcd(b, m) != 1 then
        return nil
    else
        inv = mod_inverse(b, m)
        return (a * inv) % m
    end
end

def prime_factors(n)
    if n < 2 then
        return []
    end
    factors = []
    d = 2
    while d * d <= n do
        while n % d == 0 do
            factors.push(d)
            n = n / d
        end
        d = d + 1
    end
    if n > 1 then
        factors.push(n)
    end
    return factors
end

def is_prime(n)
    if n < 2 then
        return false
    end
    if n == 2 then
        return true
    end
    if n % 2 == 0 then
        return false
    end
    i = 3
    while i * i <= n do
        if n % i == 0 then
            return false
        end
        i = i + 2
    end
    return true
end

def factorize_phi(m)
    if m == 1 then
        return []
    end
    factors = prime_factors(m)
    prime_counts = []
    i = 0
    while i < factors.length do
        p = factors[i]
        count = 0
        while i < factors.length && factors[i] == p do
            count = count + 1
            i = i + 1
        end
        if count > 1 then
            j = 0
            while j < count - 1 do
                prime_counts.push(p)
                j = j + 1
            end
        end
        p_minus_1_factors = prime_factors(p - 1)
        j = 0
        while j < p_minus_1_factors.length do
            prime_counts.push(p_minus_1_factors[j])
            j = j + 1
        end
    end
    prime_counts = sort_integers(prime_counts)
    result = []
    i = 0
    while i < prime_counts.length do
        p = prime_counts[i]
        count = 0
        while i < prime_counts.length && prime_counts[i] == p do
            count = count + 1
            i = i + 1
        end
        result.push([p, count])
    end
    return result
end

def sort_integers(arr)
    n = arr.length
    if n <= 1 then
        return arr
    end
    sorted = []
    i = 0
    while i < n do
        sorted.push(arr[i])
        i = i + 1
    end
    i = 0
    while i < n - 1 do
        j = 0
        while j < n - i - 1 do
            if sorted[j] > sorted[j + 1] then
                temp = sorted[j]
                sorted[j] = sorted[j + 1]
                sorted[j + 1] = temp
            end
            j = j + 1
        end
        i = i + 1
    end
    return sorted
end

def isqrt(n)
    if n == 0 then
        return 0
    end
    x = n
    while true do
        x1 = (x + n / x) / 2
        if x1 >= x then
            return x
        end
        x = x1
    end
end

def euler_phi(m)
    if m == 1 then
        return 1
    end
    result = m
    factors = prime_factors(m)
    unique_primes = []
    i = 0
    while i < factors.length do
        p = factors[i]
        if unique_primes.length == 0 || unique_primes[unique_primes.length - 1] != p then
            unique_primes.push(p)
        end
        i = i + 1
    end
    i = 0
    while i < unique_primes.length do
        p = unique_primes[i]
        result = result - result / p
        i = i + 1
    end
    return result
end

def is_unit(a, m)
    return gcd(a % m, m) == 1
end

def units_mod_m(m)
    units = []
    i = 0
    while i < m do
        if is_unit(i, m) then
            units.push(i)
        end
        i = i + 1
    end
    return units
end

def group_order(m)
    return euler_phi(m)
end

def element_order(a, m)
    if !is_unit(a, m) then
        raise("element_order: element is not a unit mod m")
    end
    phi = euler_phi(m)
    order = 1
    while order <= phi do
        if mod_pow(a, order, m) == 1 then
            if phi % order == 0 then
                return order
            end
        end
        order = order + 1
    end
    return raise("element_order: failed to find order (internal error)")
end

def is_cyclic(m)
    if m == 1 || m == 2 || m == 4 then
        return true
    end
    if m % 2 == 1 then
        factors = prime_factors(m)
        if factors.length == 0 then
            return false
        end
        first = factors[0]
        i = 1
        while i < factors.length do
            if factors[i] != first then
                return false
            end
            i = i + 1
        end
        return true
    end
    if m % 2 == 0 then
        m_half = m / 2
        if m_half % 2 == 1 then
            factors = prime_factors(m_half)
            if factors.length == 0 then
                return false
            end
            first = factors[0]
            i = 1
            while i < factors.length do
                if factors[i] != first then
                    return false
                end
                i = i + 1
            end
            return true
        end
    end
    return false
end

def primitive_root(m)
    if !is_cyclic(m) then
        raise("primitive_root: group is not cyclic")
    end
    phi = euler_phi(m)
    phi_factors = prime_factors(phi)
    unique_phi_primes = []
    i = 0
    while i < phi_factors.length do
        p = phi_factors[i]
        if unique_phi_primes.length == 0 || unique_phi_primes[unique_phi_primes.length - 1] != p then
            unique_phi_primes.push(p)
        end
        i = i + 1
    end
    a = 2
    while a < m do
        if is_unit(a, m) then
            is_generator = true
            i = 0
            while i < unique_phi_primes.length do
                p = unique_phi_primes[i]
                if mod_pow(a, phi / p, m) == 1 then
                    is_generator = false
                end
                i = i + 1
            end
            if is_generator then
                return a
            end
        end
        a = a + 1
    end
    return raise("primitive_root: failed to find generator (internal error)")
end

def all_primitive_roots(m)
    if !is_cyclic(m) then
        raise("all_primitive_roots: group is not cyclic")
    end
    phi = euler_phi(m)
    g = primitive_root(m)
    generators = []
    k = 1
    while k < phi do
        if gcd(k, phi) == 1 then
            generators.push(mod_pow(g, k, m))
        end
        k = k + 1
    end
    return generators
end

def discrete_log(base, value, m)
    if !is_unit(base, m) then
        raise("discrete_log: base is not a unit mod m")
    end
    if !is_unit(value, m) then
        raise("discrete_log: value is not a unit mod m")
    end
    phi = euler_phi(m)
    n = isqrt(phi) + 1
    baby_steps = []
    current = 1
    j = 0
    while j < n do
        baby_steps.push([current, j])
        current = mod_mult(current, base, m)
        j = j + 1
    end
    base_inv = mod_inverse(base, m)
    giant_step = mod_pow(base_inv, n, m)
    gamma = value
    i = 0
    while i < n do
        j = 0
        while j < baby_steps.length do
            if baby_steps[j][0] == gamma then
                result = baby_steps[j][1] + n * i
                if result < phi then
                    return result
                end
            end
            j = j + 1
        end
        gamma = mod_mult(gamma, giant_step, m)
        i = i + 1
    end
    return raise("discrete_log: no solution found")
end

def legendre_symbol(a, p)
    if !is_prime(p) || p == 2 then
        raise("legendre_symbol: p must be an odd prime")
    end
    a = a % p
    if a == 0 then
        return 0
    end
    result = mod_pow(a, (p - 1) / 2, p)
    if result == 1 then
        return 1
    else
        return -1
    end
end

def jacobi_symbol(a, n)
    if n <= 0 || n % 2 == 0 then
        raise("jacobi_symbol: n must be a positive odd integer")
    end
    if n == 1 then
        return 1
    end
    a = a % n
    result = 1
    while a != 0 do
        while a % 2 == 0 do
            a = a / 2
            n_mod_8 = n % 8
            if n_mod_8 == 3 || n_mod_8 == 5 then
                result = -result
            end
        end
        temp = a
        a = n
        n = temp
        if a % 4 == 3 && n % 4 == 3 then
            result = -result
        end
        a = a % n
    end
    if n == 1 then
        return result
    else
        return 0
    end
end

def kronecker_symbol(a, n)
    if n == 0 then
        if a == 1 || a == -1 then
            return 1
        else
            return 0
        end
    end
    if n < 0 then
        if a < 0 then
            return -kronecker_symbol(a, -n)
        else
            return kronecker_symbol(a, -n)
        end
    end
    if n == 1 then
        return 1
    end
    e = 0
    n_odd = n
    while n_odd % 2 == 0 do
        e = e + 1
        n_odd = n_odd / 2
    end
    if e > 0 then
        a_mod_8 = ((a % 8) + 8) % 8
        if e == 1 then
            if a % 2 == 0 then
                symbol_2 = 0
            elsif a_mod_8 == 1 || a_mod_8 == 7 then
                symbol_2 = 1
            else
                symbol_2 = -1
            end
        elsif a % 2 == 0 then
            symbol_2 = 0
        elsif a_mod_8 == 1 || a_mod_8 == 7 then
            symbol_2 = 1
        else
            symbol_2 = -1
        end
        if symbol_2 == 0 then
            return 0
        end
    else
        symbol_2 = 1
    end
    if n_odd == 1 then
        symbol_odd = 1
    else
        symbol_odd = jacobi_symbol(a, n_odd)
    end
    return symbol_2 * symbol_odd
end

def dirichlet_characters(m)
    if m == 1 then
        return [[]]
    end
    phi = euler_phi(m)
    units = units_mod_m(m)
    principal = []
    a = 1
    while a < m do
        if is_unit(a, m) then
            principal.push(1)
        else
            principal.push(0)
        end
        a = a + 1
    end
    return [principal]
end
