// The Lumen library file lib_lumen/number_theory.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function gcd(a, b) {
    let t;
    while (b !== 0) {
        t = b;
        b = a % b;
        a = t;
    }
    return a;
}

function is_coprime(a, b) {
    return gcd(a, b) === 1;
}

function is_prime(n) {
    if (n < 2) {
        return false;
    }
    if (n === 2) {
        return true;
    }
    if (n % 2 === 0) {
        return false;
    }
    let i = 3;
    while (i * i <= n) {
        if (n % i === 0) {
            return false;
        }
        i = i + 2;
    }
    return true;
}

function sort_integers(arr) {
    let j;
    let temp;
    const n = arr.length;
    if (n <= 1) {
        return arr;
    }
    const sorted = [];
    let i = 0;
    while (i < n) {
        sorted.push(arr[i]);
        i = i + 1;
    }
    i = 0;
    while (i < n - 1) {
        j = 0;
        while (j < n - i - 1) {
            if (sorted[j] > sorted[j + 1]) {
                temp = sorted[j];
                sorted[j] = sorted[j + 1];
                sorted[j + 1] = temp;
            }
            j = j + 1;
        }
        i = i + 1;
    }
    return sorted;
}

function is_unit(a, m) {
    return gcd(a % m, m) === 1;
}

function units_mod_m(m) {
    const units = [];
    let i = 0;
    while (i < m) {
        if (is_unit(i, m)) {
            units.push(i);
        }
        i = i + 1;
    }
    return units;
}
