# Lumen Language Quick Reference Card

## Built-in Functions and Library Reference

---

## I/O Functions

### `emit(string)` [KERNEL]
- **Description**: Kernel primitive for raw string output
- **Arguments**: `string` - A string value (no implicit conversion)
- **Returns**: `none`
- **Side Effects**: Writes string directly to stdout without formatting or newline
- **Example**: `emit("Hello")`

### `write(x)` [LIBRARY: output.lm]
- **Description**: Output with type conversion, no newline
- **Arguments**: `x` - Any value (converted to string via `str()`)
- **Returns**: `none`
- **Example**: `write(42)` outputs `42`

### `print(x)` [LIBRARY: output.lm]
- **Description**: Output with type conversion and newline
- **Arguments**: `x` - Any value (converted to string via `str()`)
- **Returns**: `none`
- **Example**: `print("Hello")` outputs `Hello\n`

---

## Operators

### Arithmetic Operators
- `+` Addition
- `-` Subtraction
- `*` Multiplication
- `/` Division (returns RATIONAL for integers, REAL for reals)
- `//` Integer division (quotient)
- `%` Modulo (remainder)
- `-` Unary negation

### Comparison Operators
- `==` Equal
- `!=` Not equal
- `<` Less than
- `<=` Less than or equal
- `>` Greater than
- `>=` Greater than or equal

### Logical Operators
- `and` Logical AND
- `or` Logical OR
- `not` Logical NOT

---

## Control Flow

### If-Else
```lumen
if condition
    # code
else
    # code
```

### While Loop
```lumen
while condition
    # code
```

### For Loop
```lumen
for variable in array
    # code
```

### Until Loop
```lumen
until condition
    # code
```

### Control Flow Keywords
- `break` - Exit loop
- `continue` - Skip to next iteration
- `return value` - Return from function

---

## Function Definition

```lumen
fn function_name(param1, param2)
    # function body
    return result  # optional explicit return
    # or implicit return of last expression
```

---

## Variable Declaration

### Immutable Binding
```lumen
let x = 42
```

### Mutable Binding
```lumen
let mut x = 42
x = 50  # Can reassign
```

### Simple Assignment
```lumen
x = 42  # No let/mut required for assignment
```

---

## Type Conversion Functions

### `int(x)` [KERNEL]
- **Description**: Extract integer part from real number
- **Arguments**: `x` - A REAL value
- **Returns**: `INTEGER`
- **Constraint**: Only works on REAL values
- **Example**: `int(3.14)` → `3`

### `real(x)` or `real(x, precision)` [KERNEL]
- **Description**: Convert to real number with configurable precision
- **Arguments**:
  - `x` - An INTEGER, RATIONAL, or REAL value
  - `precision` - Significant digits (default: 15)
- **Returns**: `REAL`
- **Example**: `real(22/7, 10)` → `3.142857143` (10 sig figs)

---

## String Functions

### `str(x)` [KERNEL]
- **Description**: Convert any value to its string representation
- **Arguments**: `x` - Any value
- **Returns**: `STRING`
- **Example**: `str(42)` → `"42"`

### String Concatenation: `.` [KERNEL]
- **Description**: Concatenate two strings using the period operator
- **Arguments**: Two STRING values
- **Returns**: `STRING`
- **Example**: `"hello" . " " . "world"` → `"hello world"`

### `len(x)` [KERNEL]
- **Description**: Return length of string or array
- **Arguments**: `x` - A STRING or ARRAY value
- **Returns**: `INTEGER`
- **Example**: `len("hello")` → `5`, `len([1,2,3])` → `3`

### `char_at(string, index)` [KERNEL]
- **Description**: Return character at zero-based index
- **Arguments**:
  - `string` - A STRING value
  - `index` - An INTEGER (zero-based)
- **Returns**: `STRING` (single character) or `none` if out of bounds
- **Example**: `char_at("hello", 1)` → `"e"`

### `ord(s)` [KERNEL]
- **Description**: Return Unicode code point of first character
- **Arguments**: `s` - A non-empty STRING
- **Returns**: `INTEGER`
- **Example**: `ord("A")` → `65`

### `chr(n)` [KERNEL]
- **Description**: Return character for Unicode code point
- **Arguments**: `n` - An INTEGER (valid Unicode code point)
- **Returns**: `STRING` (single character)
- **Example**: `chr(65)` → `"A"`

### `substring(s, start, end)` [LIBRARY: string.lm]
- **Description**: Extract substring from start (inclusive) to end (exclusive)
- **Arguments**:
  - `s` - A STRING
  - `start` - Start index (inclusive)
  - `end` - End index (exclusive)
- **Returns**: `STRING`
- **Example**: `substring("hello", 1, 4)` → `"ell"`

### `slice(s, start)` [LIBRARY: string.lm]
- **Description**: Extract substring from start index to end of string
- **Arguments**:
  - `s` - A STRING
  - `start` - Start index
- **Returns**: `STRING`
- **Example**: `slice("hello", 2)` → `"llo"`

### `starts_with(s, prefix)` [LIBRARY: string.lm]
- **Description**: Check if string begins with prefix
- **Arguments**:
  - `s` - A STRING
  - `prefix` - A STRING
- **Returns**: `BOOLEAN`
- **Example**: `starts_with("hello", "he")` → `true`

### `ends_with(s, suffix)` [LIBRARY: string.lm]
- **Description**: Check if string ends with suffix
- **Arguments**:
  - `s` - A STRING
  - `suffix` - A STRING
- **Returns**: `BOOLEAN`
- **Example**: `ends_with("hello", "lo")` → `true`

### `repeat(s, count)` [LIBRARY: string.lm]
- **Description**: Repeat string count times
- **Arguments**:
  - `s` - A STRING
  - `count` - An INTEGER
- **Returns**: `STRING`
- **Example**: `repeat("ab", 3)` → `"ababab"`

### `join(arr, sep)` [LIBRARY: string.lm]
- **Description**: Join array of strings with separator
- **Arguments**:
  - `arr` - An ARRAY of strings
  - `sep` - Separator STRING
- **Returns**: `STRING`
- **Example**: `join(["a", "b", "c"], ",")` → `"a,b,c"`

### `index_of(s, needle)` [LIBRARY: string.lm]
- **Description**: Find first index of substring, or -1 if not found
- **Arguments**:
  - `s` - A STRING to search in
  - `needle` - A STRING to search for
- **Returns**: `INTEGER` (index or -1)
- **Example**: `index_of("hello", "ll")` → `2`

### `contains(s, needle)` [LIBRARY: string.lm]
- **Description**: Check if needle occurs anywhere in string
- **Arguments**:
  - `s` - A STRING
  - `needle` - A STRING
- **Returns**: `BOOLEAN`
- **Example**: `contains("hello", "ll")` → `true`

---

## Type Introspection Functions

### `kind(x)` [KERNEL]
- **Description**: Return kind meta-value representing value category
- **Arguments**: `x` - Any value
- **Returns**: KIND (one of: INTEGER, RATIONAL, REAL, STRING, ARRAY, BOOLEAN, NONE)
- **Example**: `kind(42)` → `INTEGER`, `kind(1/2)` → `RATIONAL`
- **Use Case**: Type checking in conditionals: `if kind(x) == INTEGER`

### `num(x)` [KERNEL]
- **Description**: Extract numerator from rational number
- **Arguments**: `x` - A RATIONAL value
- **Returns**: `INTEGER`
- **Constraint**: Only works on RATIONAL values
- **Example**: `num(22/7)` → `22`

### `den(x)` [KERNEL]
- **Description**: Extract denominator from rational number
- **Arguments**: `x` - A RATIONAL value
- **Returns**: `INTEGER`
- **Constraint**: Only works on RATIONAL values
- **Example**: `den(22/7)` → `7`

### `frac(x)` [KERNEL]
- **Description**: Extract fractional part from real number
- **Arguments**: `x` - A REAL value
- **Returns**: `REAL`
- **Constraint**: Only works on REAL values
- **Property**: `int(x) + frac(x) == x`
- **Example**: `frac(3.14)` → `0.14` (as REAL)

---

## Array Functions

### `push(arr, value)` [KERNEL]
- **Description**: Append value to array (mutates in place)
- **Arguments**:
  - `arr` - Name of array variable (not expression)
  - `value` - Any value to append
- **Returns**: `none`
- **Side Effects**: Mutates the array
- **Example**: `push(mylist, 42)`

---

## Mathematical Functions

### `factorial(n)` [LIBRARY: factorial.lm]
- **Description**: Compute n! (factorial)
- **Arguments**: `n` - An INTEGER
- **Returns**: `INTEGER`
- **Example**: `factorial(5)` → `120`

### `round(x, decimals)` [LIBRARY: round.lm]
- **Description**: Round to specified decimal places (round-half-away-from-zero)
- **Arguments**:
  - `x` - A number
  - `decimals` - Number of decimal places
- **Returns**: Number rounded to specified precision
- **Example**: `round(3.14159, 2)` → `3.14`

---

## Mathematical Constants

### Core Constants [LIBRARY: constants.lm]

#### `pi(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Pi (π) with specified precision
- **Arguments**: `sigfigs` - Significant digits (max 1024)
- **Returns**: `REAL`
- **Example**: `pi(10)` → `3.141592654` (10 sig figs)

#### `e(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Euler's number (e) with specified precision
- **Arguments**: `sigfigs` - Significant digits (max 1024)
- **Returns**: `REAL`
- **Example**: `e(10)` → `2.718281828` (10 sig figs)

### Roots and Powers [LIBRARY: constants.lm]

#### `sqrt2(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Square root of 2
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `sqrt_pi(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Square root of π
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `sqrt_2pi(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Square root of 2π
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `e2(sigfigs)` [LIBRARY: constants.lm]
- **Description**: e squared (e²)
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

### Reciprocals [LIBRARY: constants.lm]

#### `inv_pi(sigfigs)` [LIBRARY: constants.lm]
- **Description**: 1/π
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `inv_e(sigfigs)` [LIBRARY: constants.lm]
- **Description**: 1/e
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `inv_sqrt_2pi(sigfigs)` [LIBRARY: constants.lm]
- **Description**: 1/√(2π)
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

### Logarithms [LIBRARY: constants.lm]

#### `ln2(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Natural logarithm of 2
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `ln10(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Natural logarithm of 10
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `log2e(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Log base 2 of e
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `log10e(sigfigs)` [LIBRARY: constants.lm]
- **Description**: Log base 10 of e
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

#### `two_over_sqrt_pi(sigfigs)` [LIBRARY: constants.lm]
- **Description**: 2/√π
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`

### Default Precision Constants [LIBRARY: constants_default.lm]
All constants have `_default()` versions that use 15 significant digits:
- `pi_default()`, `e_default()`, `sqrt2_default()`, `sqrt_pi_default()`, `sqrt_2pi_default()`, `e2_default()`
- `inv_pi_default()`, `inv_e_default()`, `inv_sqrt_2pi_default()`
- `ln2_default()`, `ln10_default()`, `log2e_default()`, `log10e_default()`
- `two_over_sqrt_pi_default()`

---

## High-Precision Mathematical Functions

### `pi_machin(sigfigs)` [LIBRARY: pi_machin.lm]
- **Description**: Calculate π using Machin's formula (pure integer arithmetic)
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`
- **Note**: Uses pure integer arithmetic with guard digits
- **Example**: `pi_machin(50)` → π to 50 significant figures

### `e_integer(sigfigs)` [LIBRARY: e_integer.lm]
- **Description**: Calculate e using Taylor series (pure integer arithmetic)
- **Arguments**: `sigfigs` - Significant digits
- **Returns**: `REAL`
- **Note**: Uses scaled integer Taylor series with guard digits
- **Example**: `e_integer(50)` → e to 50 significant figures

---

## Number Theory Functions

### `gcd(a, b)` [LIBRARY: number_theory.lm]
- **Description**: Greatest common divisor (Euclid's algorithm)
- **Arguments**: `a`, `b` - INTEGERs
- **Returns**: `INTEGER`
- **Example**: `gcd(12, 18)` → `6`

### `lcm(a, b)` [LIBRARY: number_theory.lm]
- **Description**: Least common multiple
- **Arguments**: `a`, `b` - INTEGERs
- **Returns**: `INTEGER`
- **Example**: `lcm(12, 18)` → `36`

### `is_coprime(a, b)` [LIBRARY: number_theory.lm]
- **Description**: Check if two integers are coprime (gcd = 1)
- **Arguments**: `a`, `b` - INTEGERs
- **Returns**: `BOOLEAN`
- **Example**: `is_coprime(15, 28)` → `true`

### `pow_mod(base, exp, mod)` [LIBRARY: number_theory.lm]
- **Description**: Modular exponentiation: (base^exp) % mod
- **Arguments**: `base`, `exp`, `mod` - INTEGERs
- **Returns**: `INTEGER`
- **Example**: `pow_mod(2, 10, 1000)` → `24`

### `extended_gcd(a, b)` [LIBRARY: number_theory.lm]
- **Description**: Extended Euclidean algorithm
- **Arguments**: `a`, `b` - INTEGERs
- **Returns**: `ARRAY` [g, x, y] where ax + by = g
- **Example**: `extended_gcd(240, 46)` → `[2, -9, 47]`

### `mod_inverse(a, m)` [LIBRARY: number_theory.lm]
- **Description**: Modular multiplicative inverse of a modulo m
- **Arguments**: `a`, `m` - INTEGERs
- **Returns**: `INTEGER` or `none` if inverse doesn't exist
- **Example**: `mod_inverse(3, 7)` → `5` (because 3*5 ≡ 1 mod 7)

### `mod_div(a, b, m)` [LIBRARY: number_theory.lm]
- **Description**: Modular division: (a / b) mod m
- **Arguments**: `a`, `b`, `m` - INTEGERs
- **Returns**: `INTEGER` or `none` if division impossible
- **Example**: `mod_div(10, 3, 7)` → `6`

---

## Prime Number Functions

### `is_prime(n)` [LIBRARY: primes.lm]
- **Description**: Test if number is prime (trial division)
- **Arguments**: `n` - An INTEGER
- **Returns**: `BOOLEAN`
- **Example**: `is_prime(17)` → `true`

### `next_prime(n)` [LIBRARY: primes.lm]
- **Description**: Find smallest prime greater than n
- **Arguments**: `n` - An INTEGER
- **Returns**: `INTEGER`
- **Example**: `next_prime(10)` → `11`

### `primes_up_to(limit)` [LIBRARY: primes.lm]
- **Description**: Generate all primes up to limit (Sieve of Eratosthenes)
- **Arguments**: `limit` - An INTEGER
- **Returns**: `ARRAY` of primes
- **Example**: `primes_up_to(10)` → `[2, 3, 5, 7]`

### `prime_factors(n)` [LIBRARY: primes.lm]
- **Description**: Return prime factorization as array (with repetitions)
- **Arguments**: `n` - An INTEGER
- **Returns**: `ARRAY` of prime factors
- **Example**: `prime_factors(12)` → `[2, 2, 3]`

### `unique_prime_factors(n)` [LIBRARY: primes.lm]
- **Description**: Return unique prime factors (no repetitions)
- **Arguments**: `n` - An INTEGER
- **Returns**: `ARRAY` of unique prime factors
- **Example**: `unique_prime_factors(12)` → `[2, 3]`

---

## Base Conversion Functions

### `base(value, radix)` [LIBRARY: base.lm]
- **Description**: Convert number to string representation in given base
- **Arguments**:
  - `value` - An INTEGER, RATIONAL, or REAL
  - `radix` - Base (2-36)
- **Returns**: `STRING` representation
- **Example**: `base(255, 16)` → `"ff"`
- **Supported Types**:
  - INTEGER: Full conversion
  - RATIONAL: "numerator/denominator" in target base
  - REAL: Integer and fractional parts (16 fractional digits)

### `base_integer(n, radix)` [LIBRARY: base.lm]
- **Description**: Convert integer to string in given base (helper)
- **Arguments**: `n` - INTEGER, `radix` - Base (2-36)
- **Returns**: `STRING`
- **Note**: Internal helper, use `base()` instead

### `base_rational(r, radix)` [LIBRARY: base.lm]
- **Description**: Convert rational to string in given base (helper)
- **Arguments**: `r` - RATIONAL, `radix` - Base (2-36)
- **Returns**: `STRING` "num/den"
- **Note**: Internal helper, use `base()` instead

### `base_real(x, radix)` [LIBRARY: base.lm]
- **Description**: Convert real to string in given base (helper)
- **Arguments**: `x` - REAL, `radix` - Base (2-36)
- **Returns**: `STRING` "integer.fractional"
- **Note**: Internal helper, use `base()` instead

### `base_frac(f, radix, limit)` [LIBRARY: base.lm]
- **Description**: Convert fractional part to string (helper)
- **Arguments**: `f` - REAL fractional part, `radix` - Base, `limit` - Max digits
- **Returns**: `STRING`
- **Note**: Internal helper, use `base()` instead

---

## Built-in Constants (Kind Meta-Values)

These constants are automatically defined in the global scope:

### `INTEGER` [KERNEL]
- **Type**: KIND
- **Description**: Meta-value representing the INTEGER kind
- **Usage**: `if kind(x) == INTEGER`

### `RATIONAL` [KERNEL]
- **Type**: KIND
- **Description**: Meta-value representing the RATIONAL kind
- **Usage**: `if kind(x) == RATIONAL`

### `REAL` [KERNEL]
- **Type**: KIND
- **Description**: Meta-value representing the REAL kind
- **Usage**: `if kind(x) == REAL`

### `STRING` [KERNEL]
- **Type**: KIND
- **Description**: Meta-value representing the STRING kind
- **Usage**: `if kind(x) == STRING`

### `ARRAY` [KERNEL]
- **Type**: KIND
- **Description**: Meta-value representing the ARRAY kind
- **Usage**: `if kind(x) == ARRAY`

### `BOOLEAN` [KERNEL]
- **Type**: KIND
- **Description**: Meta-value representing the BOOLEAN kind
- **Usage**: `if kind(x) == BOOLEAN`

### `NONE` [KERNEL]
- **Type**: KIND
- **Description**: Meta-value representing the NONE kind
- **Usage**: `if kind(x) == NONE`

### `ARGS` [KERNEL]
- **Type**: STRING
- **Description**: Command-line arguments as a single string
- **Usage**: Access via `ARGS` variable

---

## Error Handling

### `error(message)` [LIBRARY/USER-DEFINED]
- **Description**: Raise a runtime error with message
- **Arguments**: `message` - A STRING describing the error
- **Returns**: Never returns (terminates execution)
- **Note**: Not a built-in kernel function; typically defined in user code or library
- **Example**: `error("Invalid input")`
- **Common Pattern**:
  ```lumen
  if condition
      error("Error message")
  ```

---

## Value Types

Lumen supports the following value types:

1. **INTEGER** - Arbitrary-precision integers (BigInt)
   - Example: `42`, `-17`, `1000000000000000000`

2. **RATIONAL** - Exact rational numbers (numerator/denominator)
   - Example: `1/2`, `22/7`
   - Access components: `num(1/2)` → `1`, `den(1/2)` → `2`

3. **REAL** - Fixed-point decimals with configurable precision
   - Example: `3.14`, `2.718`
   - Access components: `int(3.14)` → `3`, `frac(3.14)` → `0.14`

4. **STRING** - UTF-8 text strings
   - Example: `"hello"`, `"世界"`

5. **BOOLEAN** - True/false values
   - Example: `true`, `false`

6. **ARRAY** - Ordered collections
   - Example: `[1, 2, 3]`, `["a", "b"]`

7. **NONE** - Null/void value
   - Example: `none`

---

## Notes

1. **No Implicit Type Conversion**: Lumen uses explicit type conversions only
2. **Arbitrary Precision**: INTEGER and RATIONAL types support unlimited precision
3. **Pure Functions**: Most library functions are pure (no side effects)
4. **Zero-Based Indexing**: Arrays and strings use zero-based indexing
5. **Indentation**: Lumen uses Python-style indentation for block structure
6. **Comments**: Use `#` for single-line comments

---

## Library Organization

- **output.lm** - I/O functions (write, print)
- **string.lm** - String manipulation functions
- **base.lm** - Base conversion utilities
- **factorial.lm** - Factorial computation
- **round.lm** - Rounding utilities
- **pi_machin.lm** - High-precision π calculation
- **e_integer.lm** - High-precision e calculation
- **constants.lm** - Mathematical constants (wrapper)
- **constants_1024.lm** - 1024-digit precision constants
- **constants_default.lm** - Default precision (15 digits) constants
- **number_theory.lm** - Number theory functions (gcd, lcm, modular arithmetic)
- **primes.lm** - Prime number functions

---

**Version**: Lumen-Lang 0.0.1
**Kernel Implementations**: Stream (tree-walking AST), Microcode (4-stage pipeline)
**Documentation**: See `/docs` for detailed design documents
