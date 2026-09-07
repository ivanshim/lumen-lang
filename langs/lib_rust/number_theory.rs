// The Lumen library file langs/lib_lumen/number_theory.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

fn gcd(a: i64, b: i64) -> i64 {
    let mut t;
    while b != 0 {
        t = b;
        b = a % b;
        a = t;
    }
    return a;
}

fn lcm(a: i64, b: i64) -> i64 {
    return (a * b) / gcd(a, b);
}

fn is_coprime(a: i64, b: i64) -> bool {
    return gcd(a, b) == 1;
}

fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i = i + 2;
    }
    return true;
}

fn isqrt(n: i64) -> i64 {
    let mut x1;
    if n == 0 {
        return 0;
    }
    let mut x = n;
    while true {
        x1 = (x + n / x) / 2;
        if x1 >= x {
            return x;
        }
        x = x1;
    }
}

fn is_unit(a: i64, m: i64) -> bool {
    return gcd(a % m, m) == 1;
}

fn legendre_symbol(a: i64, p: i64) -> i64 {
    if !is_prime(p) || p == 2 {
        panic!("legendre_symbol: p must be an odd prime");
    }
    a = a % p;
    if a == 0 {
        return 0;
    }
    let result = mod_pow(a, (p - 1) / 2, p);
    if result == 1 {
        return 1;
    } else {
        return -1;
    }
}

fn jacobi_symbol(a: i64, n: i64) -> i64 {
    let mut n_mod_8;
    let mut temp;
    if n <= 0 || n % 2 == 0 {
        panic!("jacobi_symbol: n must be a positive odd integer");
    }
    if n == 1 {
        return 1;
    }
    a = a % n;
    let mut result = 1;
    while a != 0 {
        while a % 2 == 0 {
            a = a / 2;
            n_mod_8 = n % 8;
            if n_mod_8 == 3 || n_mod_8 == 5 {
                result = -result;
            }
        }
        temp = a;
        a = n;
        n = temp;
        if a % 4 == 3 && n % 4 == 3 {
            result = -result;
        }
        a = a % n;
    }
    if n == 1 {
        return result;
    } else {
        return 0;
    }
}

fn kronecker_symbol(a: i64, n: i64) -> i64 {
    let mut a_mod_8;
    let mut symbol_2;
    let mut symbol_odd;
    if n == 0 {
        if a == 1 || a == -1 {
            return 1;
        } else {
            return 0;
        }
    }
    if n < 0 {
        if a < 0 {
            return -kronecker_symbol(a, -n);
        } else {
            return kronecker_symbol(a, -n);
        }
    }
    if n == 1 {
        return 1;
    }
    let mut e = 0;
    let mut n_odd = n;
    while n_odd % 2 == 0 {
        e = e + 1;
        n_odd = n_odd / 2;
    }
    if e > 0 {
        a_mod_8 = ((a % 8) + 8) % 8;
        if e == 1 {
            if a % 2 == 0 {
                symbol_2 = 0;
            } else if a_mod_8 == 1 || a_mod_8 == 7 {
                symbol_2 = 1;
            } else {
                symbol_2 = -1;
            }
        } else if a % 2 == 0 {
            symbol_2 = 0;
        } else if a_mod_8 == 1 || a_mod_8 == 7 {
            symbol_2 = 1;
        } else {
            symbol_2 = -1;
        }
        if symbol_2 == 0 {
            return 0;
        }
    } else {
        symbol_2 = 1;
    }
    if n_odd == 1 {
        symbol_odd = 1;
    } else {
        symbol_odd = jacobi_symbol(a, n_odd);
    }
    return symbol_2 * symbol_odd;
}
