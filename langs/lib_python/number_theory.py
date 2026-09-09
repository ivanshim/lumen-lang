# The Lumen library file langs/lib_lumen/number_theory.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def gcd(a, b):
    while b != 0:
        t = b
        b = a % b
        a = t
    return a

def extended_gcd(a, b):
    if b == 0:
        return [a, 1, 0]
    else:
        r = extended_gcd(a= b, b= a % b)
        g = r[0]
        x = r[2]
        y = r[1] - (a // b) * r[2]
        return [g, x, y]

def lcm(a, b):
    return (a * b) // gcd(a= a, b= b)

def is_coprime(a, b):
    return gcd(a= a, b= b) == 1

def mod_inverse(a, m):
    r = extended_gcd(a= a, b= m)
    if r[0] != 1:
        return sys.exit("mod_inverse: inverse does not exist (gcd(a, m) != 1)")
    else:
        return (r[1] % m + m) % m

def mod_div(a, b, m):
    if gcd(a= b, b= m) != 1:
        return None
    else:
        inv = mod_inverse(a= b, m= m)
        return (a * inv) % m

def prime_factors(n):
    if n < 2:
        return []
    factors = []
    d = 2
    while d * d <= n:
        while n % d == 0:
            factors.append(d)
            n = n // d
        d = d + 1
    if n > 1:
        factors.append(n)
    return factors

def is_prime(n):
    if n < 2:
        return False
    if n == 2:
        return True
    if n % 2 == 0:
        return False
    i = 3
    while i * i <= n:
        if n % i == 0:
            return False
        i = i + 2
    return True

def factorize_phi(m):
    if m == 1:
        return []
    factors = prime_factors(n= m)
    prime_counts = []
    i = 0
    while i < len(factors):
        p = factors[i]
        count = 0
        while i < len(factors) and factors[i] == p:
            count = count + 1
            i = i + 1
        if count > 1:
            j = 0
            while j < count - 1:
                prime_counts.append(p)
                j = j + 1
        p_minus_1_factors = prime_factors(n= p - 1)
        j = 0
        while j < len(p_minus_1_factors):
            prime_counts.append(p_minus_1_factors[j])
            j = j + 1
    prime_counts = sort_integers(arr= prime_counts)
    result = []
    i = 0
    while i < len(prime_counts):
        p = prime_counts[i]
        count = 0
        while i < len(prime_counts) and prime_counts[i] == p:
            count = count + 1
            i = i + 1
        result.append([p, count])
    return result

def sort_integers(arr):
    n = len(arr)
    if n <= 1:
        return arr
    sorted = []
    i = 0
    while i < n:
        sorted.append(arr[i])
        i = i + 1
    i = 0
    while i < n - 1:
        j = 0
        while j < n - i - 1:
            if sorted[j] > sorted[j + 1]:
                temp = sorted[j]
                sorted[j] = sorted[j + 1]
                sorted[j + 1] = temp
            j = j + 1
        i = i + 1
    return sorted

def isqrt(n):
    if n == 0:
        return 0
    x = n
    while True:
        x1 = (x + n // x) // 2
        if x1 >= x:
            return x
        x = x1

def euler_phi(m):
    if m == 1:
        return 1
    result = m
    factors = prime_factors(n= m)
    unique_primes = []
    i = 0
    while i < len(factors):
        p = factors[i]
        if len(unique_primes) == 0 or unique_primes[len(unique_primes) - 1] != p:
            unique_primes.append(p)
        i = i + 1
    i = 0
    while i < len(unique_primes):
        p = unique_primes[i]
        result = result - result // p
        i = i + 1
    return result

def is_unit(a, m):
    return gcd(a= a % m, b= m) == 1

def units_mod_m(m):
    units = []
    i = 0
    while i < m:
        if is_unit(a= i, m= m):
            units.append(i)
        i = i + 1
    return units

def group_order(m):
    return euler_phi(m= m)

def element_order(a, m):
    if not is_unit(a= a, m= m):
        sys.exit("element_order: element is not a unit mod m")
    phi = euler_phi(m= m)
    order = 1
    while order <= phi:
        if mod_pow(base= a, exp= order, m= m) == 1:
            if phi % order == 0:
                return order
        order = order + 1
    return sys.exit("element_order: failed to find order (internal error)")

def is_cyclic(m):
    if m == 1 or m == 2 or m == 4:
        return True
    if m % 2 == 1:
        factors = prime_factors(n= m)
        if len(factors) == 0:
            return False
        first = factors[0]
        i = 1
        while i < len(factors):
            if factors[i] != first:
                return False
            i = i + 1
        return True
    if m % 2 == 0:
        m_half = m // 2
        if m_half % 2 == 1:
            factors = prime_factors(n= m_half)
            if len(factors) == 0:
                return False
            first = factors[0]
            i = 1
            while i < len(factors):
                if factors[i] != first:
                    return False
                i = i + 1
            return True
    return False

def primitive_root(m):
    if not is_cyclic(m= m):
        sys.exit("primitive_root: group is not cyclic")
    phi = euler_phi(m= m)
    phi_factors = prime_factors(n= phi)
    unique_phi_primes = []
    i = 0
    while i < len(phi_factors):
        p = phi_factors[i]
        if len(unique_phi_primes) == 0 or unique_phi_primes[len(unique_phi_primes) - 1] != p:
            unique_phi_primes.append(p)
        i = i + 1
    a = 2
    while a < m:
        if is_unit(a= a, m= m):
            is_generator = True
            i = 0
            while i < len(unique_phi_primes):
                p = unique_phi_primes[i]
                if mod_pow(base= a, exp= phi // p, m= m) == 1:
                    is_generator = False
                i = i + 1
            if is_generator:
                return a
        a = a + 1
    return sys.exit("primitive_root: failed to find generator (internal error)")

def all_primitive_roots(m):
    if not is_cyclic(m= m):
        sys.exit("all_primitive_roots: group is not cyclic")
    phi = euler_phi(m= m)
    g = primitive_root(m= m)
    generators = []
    k = 1
    while k < phi:
        if gcd(a= k, b= phi) == 1:
            generators.append(mod_pow(base= g, exp= k, m= m))
        k = k + 1
    return generators

def discrete_log(base, value, m):
    if not is_unit(a= base, m= m):
        sys.exit("discrete_log: base is not a unit mod m")
    if not is_unit(a= value, m= m):
        sys.exit("discrete_log: value is not a unit mod m")
    phi = euler_phi(m= m)
    n = isqrt(n= phi) + 1
    baby_steps = []
    current = 1
    j = 0
    while j < n:
        baby_steps.append([current, j])
        current = mod_mult(a= current, b= base, m= m)
        j = j + 1
    base_inv = mod_inverse(a= base, m= m)
    giant_step = mod_pow(base= base_inv, exp= n, m= m)
    gamma = value
    i = 0
    while i < n:
        j = 0
        while j < len(baby_steps):
            if baby_steps[j][0] == gamma:
                result = baby_steps[j][1] + n * i
                if result < phi:
                    return result
            j = j + 1
        gamma = mod_mult(a= gamma, b= giant_step, m= m)
        i = i + 1
    return sys.exit("discrete_log: no solution found")

def legendre_symbol(a, p):
    if not is_prime(n= p) or p == 2:
        sys.exit("legendre_symbol: p must be an odd prime")
    a = a % p
    if a == 0:
        return 0
    result = mod_pow(base= a, exp= (p - 1) // 2, m= p)
    if result == 1:
        return 1
    else:
        return -1

def jacobi_symbol(a, n):
    if n <= 0 or n % 2 == 0:
        sys.exit("jacobi_symbol: n must be a positive odd integer")
    if n == 1:
        return 1
    a = a % n
    result = 1
    while a != 0:
        while a % 2 == 0:
            a = a // 2
            n_mod_8 = n % 8
            if n_mod_8 == 3 or n_mod_8 == 5:
                result = -result
        temp = a
        a = n
        n = temp
        if a % 4 == 3 and n % 4 == 3:
            result = -result
        a = a % n
    if n == 1:
        return result
    else:
        return 0

def kronecker_symbol(a, n):
    if n == 0:
        if a == 1 or a == -1:
            return 1
        else:
            return 0
    if n < 0:
        if a < 0:
            return -kronecker_symbol(a= a, n= -n)
        else:
            return kronecker_symbol(a= a, n= -n)
    if n == 1:
        return 1
    e = 0
    n_odd = n
    while n_odd % 2 == 0:
        e = e + 1
        n_odd = n_odd // 2
    if e > 0:
        a_mod_8 = ((a % 8) + 8) % 8
        if e == 1:
            if a % 2 == 0:
                symbol_2 = 0
            elif a_mod_8 == 1 or a_mod_8 == 7:
                symbol_2 = 1
            else:
                symbol_2 = -1
        elif a % 2 == 0:
            symbol_2 = 0
        elif a_mod_8 == 1 or a_mod_8 == 7:
            symbol_2 = 1
        else:
            symbol_2 = -1
        if symbol_2 == 0:
            return 0
    else:
        symbol_2 = 1
    if n_odd == 1:
        symbol_odd = 1
    else:
        symbol_odd = jacobi_symbol(a= a, n= n_odd)
    return symbol_2 * symbol_odd

def dirichlet_characters(m):
    if m == 1:
        return [[]]
    phi = euler_phi(m= m)
    units = units_mod_m(m= m)
    principal = []
    a = 1
    while a < m:
        if is_unit(a= a, m= m):
            principal.append(1)
        else:
            principal.append(0)
        a = a + 1
    return [principal]
