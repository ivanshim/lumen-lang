# Lumen Compact Reference Card

This card lists **user-accessible functions** across the kernel primitives and the standard Lumen library, plus core operators and syntax reminders. For each category, kernel functions are listed first, followed by library functions. Each function is tagged as `[kernel]` or `[library]`.

> **Library scope note:** All functions in `lib_lumen/*.lm` become available to users when the corresponding library file is loaded.

---

## Core Syntax, Evaluation & Control Flow

**Conditionals & Loops**
- `if` / `else`
- `while`
- `for ... in ...`
- `until`

**Flow Keywords**
- `break` Exit loop
- `continue` Next iteration
- `return value` Return from function

**System Controls**
- `MEMOIZATION = true|false` Enable/disable memoized function caching (dynamically scoped). Particularly effective for recursive functions (e.g. naive recursive Fibonacci).

**Definitions & Bindings**
- `fn name(params)` Function definition
- `let x = value` Immutable binding
- `let mut x = value` Mutable binding

---

## Operators & Expression Composition

**Arithmetic**
- `+` Addition
- `-` Subtraction
- `*` Multiplication
- `/` Division (returns `RATIONAL` for integers, `REAL` for reals)
- `**` Exponentiation
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

**Pipes & Ranges**
- `value |> fn(args...)` Pipe operator (passes `value` as the first argument).
- `start..end` Half-open range literal (evaluates to a range value).

---

## Primitive Values & Literals

**Base syntax**
- `base@digits` — Write a numeric literal in `base` (2..36), with the base value itself written in base 10. Example: `2@1011` or `16@FF`.

**Exponent syntax**
- `value**exponent` — Power operator for numeric values.

**Array Literals & Indexing**
- `[a, b, c]` — Array literal (trailing comma allowed).
- `arr[i]` — Array indexing expression.
- `arr[i] = value` — Array indexed assignment.

---

## Runtime Value Taxonomy

**Presence**
- **Atomic**
  - **Numeric** (type hierarchy)
    - INTEGER ⊆
    - RATIONAL ⊆
    - REAL ⊆
    - COMPLEX *(future implementation)*
  - **Symbolic**
    - BOOLEAN
    - STRING

- **Composite**
  - **Structural**
    - ARRAY

**Absence**
- NULL

---

## Runtime Kinds & Type Introspection

**Kernel**
- `kind(x)` — `[kernel]` Return the kind meta-value (`INTEGER`, `RATIONAL`, `REAL`, `COMPLEX` (future implementation), `BOOLEAN`, `STRING`, `ARRAY`, `NULL`).
- `INTEGER`, `RATIONAL`, `REAL`, `COMPLEX` (future implementation), `BOOLEAN`, `STRING`, `ARRAY`, `NULL` — Kind meta-values for `kind(x)` checks.
- `ARGS` — Command-line arguments as a single string.

---

## Value-String Conversions

**Kernel**
- `int_to_string(x)` — `[kernel]` Convert INTEGER to string (mechanical primitive).
- `rational_to_string(x)` — `[kernel]` Convert RATIONAL to string (mechanical primitive).
- `real_to_string(x)` — `[kernel]` Convert REAL to string (mechanical primitive).
- `bool_to_string(x)` — `[kernel]` Convert BOOLEAN to string (mechanical primitive).
- `array_to_string(x)` — `[kernel]` Convert ARRAY to string (mechanical primitive).
- `null_to_string(x)` — `[kernel]` Convert NULL to string (mechanical primitive).

**Library** (lib_lumen/value_to_string.lm)
- `is_int(x)` — `[library]` Returns `true` if `x` has INTEGER kind.
- `is_rational(x)` — `[library]` Returns `true` if `x` has RATIONAL kind.
- `is_real(x)` — `[library]` Returns `true` if `x` has REAL kind.
- `is_bool(x)` — `[library]` Returns `true` if `x` has BOOLEAN kind.
- `is_string(x)` — `[library]` Returns `true` if `x` has STRING kind.
- `is_array(x)` — `[library]` Returns `true` if `x` has ARRAY kind.
- `is_null(x)` — `[library]` Returns `true` if `x` has NULL kind.
- `kind_to_string(k)` — `[library]` Convert a KIND meta-value to its canonical uppercase string representation ("INTEGER", "REAL", etc.).
- `str(x)` — `[library]` Convert any value to its canonical string representation.
- `numeric_to_base_string(value, radix)` — `[library]` Convert integer/rational/real to a string in the given base (2..36).
- `integer_to_base_string(n, radix)` — `[library]` Base conversion for integers.
- `rational_to_base_string(r, radix)` — `[library]` Base conversion for rationals (numerator/denominator).
- `real_to_base_string(value, radix, precision)` — `[library]` Base conversion for reals with specified fractional precision.
- `real_to_base_string_default(value, radix)` — `[library]` Base conversion for reals using `REAL_DEFAULT_PRECISION`.
- `frac_to_base_string(f, radix, limit)` — `[library]` Fractional helper used by real_to_base_string.

**Library** (lib_lumen/string_to_value.lm)
- `string_to_value(s)` — `[library]` Parse string to numeric value (supports base prefixes, rationals, reals, exponents).
- `parse_number(s, i)` — `[library]` Parse numeric literal starting at index i, returns [value, new_index].
- `parse_digits(s, i, base)` — `[library]` Parse base-N digits starting at index i, returns [value, scale, new_index].
- `digit_value(c)` — `[library]` Convert character to digit value (0-35), or -1 if invalid.

---

## Numeric Structure & Decomposition

**Kernel**
- `num(x)` — `[kernel]` Numerator of a rational (errors on non-rationals).
- `den(x)` — `[kernel]` Denominator of a rational (errors on non-rationals).
- `int(x)` — `[kernel]` Integer part of a real value.
- `frac(x)` — `[kernel]` Fractional part of a real value (same precision as input).
- `REAL_DEFAULT_PRECISION = 15` — `[kernel]` Default significant-digit precision for real conversions.
- `real(x, precision)` — `[kernel]` Convert integer/rational/real to a real value with the requested significant-digit precision.

**Library** (lib_lumen/numeric.lm)
- `real_default(x)` — `[library]` Convert numeric value to real using `REAL_DEFAULT_PRECISION`.

---

## Output

**Kernel**
- `emit(string)` — `[kernel]` Write a raw string to stdout; requires a string input and returns `null`.

**Library** (lib_lumen/output.lm)
- `write(x)` — `[library]` Convert `x` to a string with `str(x)` and emit without a newline.
- `print(x)` — `[library]` Write `x` followed by a newline.

---

## Conversion, Stringification & Output

**Kernel**

---

## Strings & Text Processing

**Kernel**
- `string_a . string_b` — `[kernel]` Concatenate strings with the `.` operator.
- `len(x)` — `[kernel]` Length of a string (UTF-8 characters) or an array.
- `char_at(string, index)` — `[kernel]` Character at a zero-based index (returns `null` if out of bounds).
- `ord(string)` — `[kernel]` Unicode code point of the first character.
- `chr(integer)` — `[kernel]` Single-character string for a Unicode code point.

**Library** (lib_lumen/string.lm)
- `substring(s, from_start, to_end)` — `[library]` Slice string from `from_start` (inclusive) to `to_end` (exclusive).
- `substring_end(s, from_here)` — `[library]` Slice string from `from_here` to the end.
- `substring_start(s, to_here)` — `[library]` Slice string from the beginning to `to_here` (exclusive).
- `starts_with(s, prefix)` — `[library]` True if `s` begins with `prefix`.
- `ends_with(s, suffix)` — `[library]` True if `s` ends with `suffix`.
- `repeat_string(s, repetitions)` — `[library]` Repeat string `repetitions` times.
- `join_strings(arr, separator)` — `[library]` Join array of strings with a separator.
- `index_of(s, needle)` — `[library]` Index of first occurrence of `needle` in `s` (or `-1`).
- `has_substring(s, needle)` — `[library]` True if `needle` appears in `s`.

---

## Character Classification & String Transformation

**Kernel**
- (none)

**Library** (lib_lumen/string_ord_chr.lm)

**Character Predicates**
- `is_ascii(c)` — `[library]` True if character is ASCII (code point < 128).
- `is_digit(c)` — `[library]` True if character is a decimal digit (0-9).
- `is_alpha(c)` — `[library]` True if character is ASCII alphabetic (A-Z, a-z).
- `is_alnum(c)` — `[library]` True if character is ASCII alphanumeric.

**Character Transformation**
- `to_upper_char(c)` — `[library]` Convert ASCII character to uppercase.
- `to_lower_char(c)` — `[library]` Convert ASCII character to lowercase.

**String Transformation**
- `to_upper(s)` — `[library]` Convert string to uppercase (ASCII only).
- `to_lower(s)` — `[library]` Convert string to lowercase (ASCII only).
- `reverse(s)` — `[library]` Reverse characters in a string.
- `capitalize_first_word(s)` — `[library]` Capitalize the first word of a string (ASCII only).
- `capitalize_words(s)` — `[library]` Capitalize the first letter of each word in a string (ASCII only).
- `trim_start(s)` — `[library]` Remove leading ASCII whitespace.
- `trim_end(s)` — `[library]` Remove trailing ASCII whitespace.
- `trim(s)` — `[library]` Remove leading and trailing ASCII whitespace.

**String Predicates**
- `is_alpha_string(s)` — `[library]` True if string consists only of ASCII letters.

---

## Arrays & Collections

**Kernel**
- `push(arr, value)` — `[kernel]` Append `value` to array `arr` (mutates in place).

**Library**
- (none)

---

## External Interaction

**Kernel**
- `extern("selector", args...)` — `[kernel]` Call an external capability (selector must be a string literal).

---

## Mathematics Libraries

**Rounding & Factorial**

**Kernel**
- (none)

**Library**
- `round(x, decimals)` — `[library]` Round using round-half-away-from-zero semantics. (lib_lumen/round.lm)
- `factorial(n)` — `[library]` Recursive integer factorial. (lib_lumen/factorial.lm)

**Number Theory**

**Kernel**
- (none)

**Library** (lib_lumen/number_theory.lm)
- `gcd(a, b)` — `[library]` Greatest common divisor (Euclid's algorithm).
- `lcm(a, b)` — `[library]` Least common multiple.
- `is_coprime(a, b)` — `[library]` True if `a` and `b` are coprime.
- `pow_mod(base, exp, mod)` — `[library]` Fast modular exponentiation.
- `extended_gcd(a, b)` — `[library]` Returns `[g, x, y]` where `ax + by = g`.
- `mod_inverse(a, m)` — `[library]` Modular inverse or `null` if it does not exist.
- `mod_div(a, b, m)` — `[library]` Modular division `a / b (mod m)` using `mod_inverse`.

**Prime Utilities**

**Kernel**
- (none)

**Library** (lib_lumen/primes.lm)
- `is_prime(n)` — `[library]` Trial-division primality test.
- `next_prime(n)` — `[library]` Smallest prime greater than `n`.
- `primes_up_to(limit)` — `[library]` Sieve of Eratosthenes, inclusive.
- `prime_factors(n)` — `[library]` Prime factorization (with repeats).
- `unique_prime_factors(n)` — `[library]` Unique prime factors of `n`.

---

## Constants with 1024-Digit Backing Stores

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

Parameterised precision is supported via the `sigfigs` parameter in functions such as `pi(sigfigs)`, `e(sigfigs)`, etc. (lib_lumen/constants.lm).

Default-precision helpers such as `pi_default()`, `e_default()`, etc. derive their precision from `REAL_DEFAULT_PRECISION` (lib_lumen/constants_default.lm).

---

## Series-Based Constants

**Kernel**
- (none)

**Library**
- `pi_machin(sigfigs)` — `[library]` π via Machin's formula (integer arithmetic). (lib_lumen/pi_machin.lm)
- `e_integer(sigfigs)` — `[library]` e via integer Taylor series with guard digits. (lib_lumen/e_integer.lm)
