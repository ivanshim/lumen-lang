# Lumen Compact Reference Card

This card lists **user-accessible functions** across the kernel primitives and the standard Lumen library, plus core operators and syntax reminders. For each category, kernel functions are listed first, followed by library functions. Each function is tagged as `[kernel]` or `[library]`.

> **Library scope note:** All functions in `lib_lumen/*.lm` become available to users when the corresponding library file is loaded.

---

## Output / I-O

**Kernel**
- `emit(string)` — `[kernel]` Write a raw string to stdout; requires a string input and returns `none`.

**Library** (lib_lumen/output.lm)
- `write(x)` — `[library]` Convert `x` to a string with `str(x)` and emit without a newline.
- `print(x)` — `[library]` Write `x` followed by a newline (implemented via `write`).

---

## Type Conversion & Introspection

**Kernel**
- `real(x)` — `[kernel]` Convert integer/rational/real to a real value using default precision (15 sig figs).
- `real(x, precision)` — `[kernel]` Convert to real with the requested significant-digit precision.
- `kind(x)` — `[kernel]` Return the kind meta-value (`INTEGER`, `RATIONAL`, `REAL`, `ARRAY`, `STRING`, `BOOLEAN`, `NONE`).
- `num(x)` — `[kernel]` Numerator of a rational (errors on non-rationals).
- `den(x)` — `[kernel]` Denominator of a rational (errors on non-rationals).
- `int(x)` — `[kernel]` Integer part of a real value.
- `frac(x)` — `[kernel]` Fractional part of a real value (same precision as input).

**Library**
- (none)

---

## String Functions

**Kernel**
- `str(x)` — `[kernel]` Convert any value to its string representation.
- `len(x)` — `[kernel]` Length of a string (UTF-8 characters) or an array.
- `char_at(string, index)` — `[kernel]` Character at a zero-based index (returns `none` if out of bounds).
- `ord(string)` — `[kernel]` Unicode code point of the first character.
- `chr(integer)` — `[kernel]` Single-character string for a Unicode code point.
- `string_a . string_b` — `[kernel]` Concatenate strings with the `.` operator.

**Library** (lib_lumen/string.lm)
- `substring(s, start, end)` — `[library]` Slice string from `start` (inclusive) to `end` (exclusive).
- `slice(s, start)` — `[library]` Slice string from `start` to the end.
- `starts_with(s, prefix)` — `[library]` True if `s` begins with `prefix`.
- `ends_with(s, suffix)` — `[library]` True if `s` ends with `suffix`.
- `repeat(s, count)` — `[library]` Repeat string `count` times.
- `join(arr, sep)` — `[library]` Join array of strings with a separator.
- `index_of(s, needle)` — `[library]` Index of first occurrence of `needle` in `s` (or `-1`).
- `contains(s, needle)` — `[library]` True if `needle` appears in `s`.

---

## Array Functions

**Kernel**
- `push(arr, value)` — `[kernel]` Append `value` to array `arr` (mutates in place).

**Library**
- (none)

---

## Operators [kernel]

**Arithmetic**
- `+` Addition
- `-` Subtraction
- `*` Multiplication
- `/` Division (returns `RATIONAL` for integers, `REAL` for reals)
- `//` Integer division (quotient)
- `%` Modulo (remainder)
- `-` Unary negation

**Comparison**
- `==` Equal
- `!=` Not equal
- `<` Less than
- `<=` Less than or equal
- `>` Greater than
- `>=` Greater than or equal

**Logical**
- `and` Logical AND
- `or` Logical OR
- `not` Logical NOT

---

## Control Flow & Definitions [kernel]

**Conditionals & Loops**
- `if` / `else`
- `while`
- `for ... in ...`
- `until`

**Flow Keywords**
- `break` Exit loop
- `continue` Next iteration
- `return value` Return from function

**Definitions & Bindings**
- `fn name(params)` Function definition
- `let x = value` Immutable binding
- `let mut x = value` Mutable binding

---

## Built-in Kind Constants & Globals

**Kernel**
- `INTEGER`, `RATIONAL`, `REAL`, `STRING`, `ARRAY`, `BOOLEAN`, `NONE` — Kind meta-values for `kind(x)` checks.
- `ARGS` — Command-line arguments as a single string.

**Library**
- (none)

---

## Numeric Formatting (Bases)

**Kernel**
- (none)

**Library** (lib_lumen/base.lm)
- `base(value, radix)` — `[library]` Convert integer/rational/real to a string in the given base (2..36).
- `base_integer(n, radix)` — `[library]` Base conversion for integers.
- `base_rational(r, radix)` — `[library]` Base conversion for rationals (numerator/denominator).
- `base_real(x, radix)` — `[library]` Base conversion for reals with fixed fractional precision.
- `base_frac(f, radix, limit)` — `[library]` Fractional helper used by `base_real`.

---

## Rounding & Factorial

**Kernel**
- (none)

**Library**
- `round(x, decimals)` — `[library]` Round using round-half-away-from-zero semantics. (lib_lumen/round.lm)
- `factorial(n)` — `[library]` Recursive integer factorial. (lib_lumen/factorial.lm)

---

## Number Theory

**Kernel**
- (none)

**Library** (lib_lumen/number_theory.lm)
- `gcd(a, b)` — `[library]` Greatest common divisor (Euclid’s algorithm).
- `lcm(a, b)` — `[library]` Least common multiple.
- `is_coprime(a, b)` — `[library]` True if `a` and `b` are coprime.
- `pow_mod(base, exp, mod)` — `[library]` Fast modular exponentiation.
- `extended_gcd(a, b)` — `[library]` Returns `[g, x, y]` where `ax + by = g`.
- `mod_inverse(a, m)` — `[library]` Modular inverse or `none` if it does not exist.
- `mod_div(a, b, m)` — `[library]` Modular division `a / b (mod m)` using `mod_inverse`.

---

## Prime Utilities

**Kernel**
- (none)

**Library** (lib_lumen/primes.lm)
- `is_prime(n)` — `[library]` Trial-division primality test.
- `next_prime(n)` — `[library]` Smallest prime greater than `n`.
- `primes_up_to(limit)` — `[library]` Sieve of Eratosthenes, inclusive.
- `prime_factors(n)` — `[library]` Prime factorization (with repeats).
- `unique_prime_factors(n)` — `[library]` Unique prime factors of `n`.

---

## Constants (significant digits)

**Kernel**
- (none)

**Library** (lib_lumen/constants.lm)
- `pi(sigfigs)` — `[library]` π with `sigfigs` significant digits.
- `e(sigfigs)` — `[library]` e with `sigfigs` significant digits.
- `sqrt2(sigfigs)` — `[library]` √2 with `sigfigs` significant digits.
- `sqrt_pi(sigfigs)` — `[library]` √π with `sigfigs` significant digits.
- `sqrt_2pi(sigfigs)` — `[library]` √(2π) with `sigfigs` significant digits.
- `e2(sigfigs)` — `[library]` e² with `sigfigs` significant digits.
- `inv_pi(sigfigs)` — `[library]` 1/π with `sigfigs` significant digits.
- `inv_e(sigfigs)` — `[library]` 1/e with `sigfigs` significant digits.
- `inv_sqrt_2pi(sigfigs)` — `[library]` 1/√(2π) with `sigfigs` significant digits.
- `ln2(sigfigs)` — `[library]` ln(2) with `sigfigs` significant digits.
- `ln10(sigfigs)` — `[library]` ln(10) with `sigfigs` significant digits.
- `log2e(sigfigs)` — `[library]` log₂(e) with `sigfigs` significant digits.
- `log10e(sigfigs)` — `[library]` log₁₀(e) with `sigfigs` significant digits.
- `two_over_sqrt_pi(sigfigs)` — `[library]` 2/√π with `sigfigs` significant digits.

---

## Constants (default precision)

**Kernel**
- (none)

**Library** (lib_lumen/constants_default.lm)
- `pi_default()` — `[library]` π with default precision (15 sig figs).
- `e_default()` — `[library]` e with default precision (15 sig figs).
- `sqrt2_default()` — `[library]` √2 with default precision (15 sig figs).
- `sqrt_pi_default()` — `[library]` √π with default precision (15 sig figs).
- `sqrt_2pi_default()` — `[library]` √(2π) with default precision (15 sig figs).
- `e2_default()` — `[library]` e² with default precision (15 sig figs).
- `inv_pi_default()` — `[library]` 1/π with default precision (15 sig figs).
- `inv_e_default()` — `[library]` 1/e with default precision (15 sig figs).
- `inv_sqrt_2pi_default()` — `[library]` 1/√(2π) with default precision (15 sig figs).
- `ln2_default()` — `[library]` ln(2) with default precision (15 sig figs).
- `ln10_default()` — `[library]` ln(10) with default precision (15 sig figs).
- `log2e_default()` — `[library]` log₂(e) with default precision (15 sig figs).
- `log10e_default()` — `[library]` log₁₀(e) with default precision (15 sig figs).
- `two_over_sqrt_pi_default()` — `[library]` 2/√π with default precision (15 sig figs).

---

## Constants (1024-digit backing)

**Kernel**
- (none)

**Library** (lib_lumen/constants_1024.lm)
- `real_from_const(sigfigs, max_sigfigs, scaled)` — `[library]` Helper to scale/round a stored integer constant into a real.
- `pi_1024(sigfigs)` — `[library]` π from a 1024-digit backing store.
- `e_1024(sigfigs)` — `[library]` e from a 1024-digit backing store.
- `sqrt2_1024(sigfigs)` — `[library]` √2 from a 1024-digit backing store.
- `sqrt_pi_1024(sigfigs)` — `[library]` √π from a 1024-digit backing store.
- `sqrt_2pi_1024(sigfigs)` — `[library]` √(2π) from a 1024-digit backing store.
- `e2_1024(sigfigs)` — `[library]` e² from a 1024-digit backing store.
- `inv_pi_1024(sigfigs)` — `[library]` 1/π from a 1024-digit backing store.
- `inv_e_1024(sigfigs)` — `[library]` 1/e from a 1024-digit backing store.
- `inv_sqrt_2pi_1024(sigfigs)` — `[library]` 1/√(2π) from a 1024-digit backing store.
- `ln2_1024(sigfigs)` — `[library]` ln(2) from a 1024-digit backing store.
- `ln10_1024(sigfigs)` — `[library]` ln(10) from a 1024-digit backing store.
- `log2e_1024(sigfigs)` — `[library]` log₂(e) from a 1024-digit backing store.
- `log10e_1024(sigfigs)` — `[library]` log₁₀(e) from a 1024-digit backing store.
- `two_over_sqrt_pi_1024(sigfigs)` — `[library]` 2/√π from a 1024-digit backing store.

---

## Series-Based Constants

**Kernel**
- (none)

**Library**
- `pi_machin(sigfigs)` — `[library]` π via Machin’s formula (integer arithmetic). (lib_lumen/pi_machin.lm)
- `e_integer(sigfigs)` — `[library]` e via integer Taylor series with guard digits. (lib_lumen/e_integer.lm)
