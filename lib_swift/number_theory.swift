// The Lumen library file lib_lumen/number_theory.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

func gcd(a: Int, b: Int) -> Int {
    var t: Int
    while b != 0 {
        t = b
        b = a % b
        a = t
    }
    return a
}

func lcm(a: Int, b: Int) -> Int {
    return (a * b) / gcd(a: a, b: b)
}

func is_coprime(a: Int, b: Int) -> Bool {
    return gcd(a: a, b: b) == 1
}

func is_prime(n: Int) -> Bool {
    if n < 2 {
        return false
    }
    if n == 2 {
        return true
    }
    if n % 2 == 0 {
        return false
    }
    var i = 3
    while i * i <= n {
        if n % i == 0 {
            return false
        }
        i = i + 2
    }
    return true
}

func isqrt(n: Int) -> Int {
    var x1: Int
    if n == 0 {
        return 0
    }
    var x = n
    while true {
        x1 = (x + n / x) / 2
        if x1 >= x {
            return x
        }
        x = x1
    }
}

func is_unit(a: Int, m: Int) -> Bool {
    return gcd(a: a % m, b: m) == 1
}

func legendre_symbol(a: Int, p: Int) -> Int {
    if !is_prime(n: p) || p == 2 {
        fatalError("legendre_symbol: p must be an odd prime")
    }
    a = a % p
    if a == 0 {
        return 0
    }
    let result = mod_pow(base: a, exp: (p - 1) / 2, m: p)
    if result == 1 {
        return 1
    } else {
        return -1
    }
}

func jacobi_symbol(a: Int, n: Int) -> Int {
    var n_mod_8: Int
    var temp: Int
    if n <= 0 || n % 2 == 0 {
        fatalError("jacobi_symbol: n must be a positive odd integer")
    }
    if n == 1 {
        return 1
    }
    a = a % n
    var result = 1
    while a != 0 {
        while a % 2 == 0 {
            a = a / 2
            n_mod_8 = n % 8
            if n_mod_8 == 3 || n_mod_8 == 5 {
                result = -result
            }
        }
        temp = a
        a = n
        n = temp
        if a % 4 == 3 && n % 4 == 3 {
            result = -result
        }
        a = a % n
    }
    if n == 1 {
        return result
    } else {
        return 0
    }
}

func kronecker_symbol(a: Int, n: Int) -> Int {
    var a_mod_8: Int
    var symbol_2: Int
    var symbol_odd: Int
    if n == 0 {
        if a == 1 || a == -1 {
            return 1
        } else {
            return 0
        }
    }
    if n < 0 {
        if a < 0 {
            return -kronecker_symbol(a: a, n: -n)
        } else {
            return kronecker_symbol(a: a, n: -n)
        }
    }
    if n == 1 {
        return 1
    }
    var e = 0
    var n_odd = n
    while n_odd % 2 == 0 {
        e = e + 1
        n_odd = n_odd / 2
    }
    if e > 0 {
        a_mod_8 = ((a % 8) + 8) % 8
        if e == 1 {
            if a % 2 == 0 {
                symbol_2 = 0
            } else if a_mod_8 == 1 || a_mod_8 == 7 {
                symbol_2 = 1
            } else {
                symbol_2 = -1
            }
        } else if a % 2 == 0 {
            symbol_2 = 0
        } else if a_mod_8 == 1 || a_mod_8 == 7 {
            symbol_2 = 1
        } else {
            symbol_2 = -1
        }
        if symbol_2 == 0 {
            return 0
        }
    } else {
        symbol_2 = 1
    }
    if n_odd == 1 {
        symbol_odd = 1
    } else {
        symbol_odd = jacobi_symbol(a: a, n: n_odd)
    }
    return symbol_2 * symbol_odd
}
