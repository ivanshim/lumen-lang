// Ported from examples/lumen/sieve.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function substring(s, from_start, to_end) {
    let index = from_start;
    let out = "";
    while (index < to_end) {
        out = out + s.charAt(index);
        index = index + 1;
    }
    return out;
}

function substring_start(s, to_here) {
    return substring(s, 0, to_here);
}

function primes_up_to(limit) {
    let k;
    const sieve = [];
    let i = 0;
    while (i <= limit) {
        sieve.push(true);
        i = i + 1;
    }
    sieve[0] = false;
    sieve[1] = false;
    let p = 2;
    while (p * p <= limit) {
        if (sieve[p]) {
            k = p * p;
            while (k <= limit) {
                sieve[k] = false;
                k = k + p;
            }
        }
        p = p + 1;
    }
    const primes = [];
    i = 2;
    while (i <= limit) {
        if (sieve[i]) {
            primes.push(i);
        }
        i = i + 1;
    }
    return primes;
}

const result = primes_up_to(10000);
const result_string = String(result);
console.log(substring_start(result_string, 100));
