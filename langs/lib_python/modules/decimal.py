# A decimal held exactly, the way its text is written: a sign, a whole
# number of digits and the power of ten they stand at, with infinity
# and the value nothing is equal to kept apart as marks of their own.
# Arithmetic here is carried out exactly; only a root, which no decimal
# answers exactly, is rounded, and only then to as many figures as the
# working needs rather than to a context's own count -- more than
# enough for anything this language converts such a root through, a
# binary real included, and never fewer.

import math


class Context:
    def __init__(self, prec=28, rounding=None, Emin=None, Emax=None, capitals=1, clamp=0, flags=None, traps=None):
        self.prec = prec
        self.rounding = rounding if rounding is not None else 'ROUND_HALF_EVEN'
        self.Emin = Emin if Emin is not None else -999999
        self.Emax = Emax if Emax is not None else 999999
        self.capitals = capitals
        self.clamp = clamp

    def copy(self):
        return Context(self.prec, self.rounding, self.Emin, self.Emax, self.capitals, self.clamp)

    def __repr__(self):
        return 'Context(prec=' + repr(self.prec) + ', rounding=' + repr(self.rounding) + ')'


_context = Context()


def getcontext():
    return _context


def setcontext(context):
    global _context
    _context = context


class localcontext:
    """A context borrowed for the length of a `with` block, and given
    back, whatever the block itself changes about the one now current."""

    def __init__(self, ctx=None):
        self.new_context = ctx.copy() if ctx is not None else getcontext().copy()

    def __enter__(self):
        self.saved_context = getcontext()
        setcontext(self.new_context)
        return self.new_context

    def __exit__(self, *exc):
        setcontext(self.saved_context)
        return False


def _gcd_int(a, b):
    while b != 0:
        a, b = b, a % b
    return a if a > 0 else -a


def _isqrt(n):
    if n <= 0:
        return 0
    x = 1 << ((n.bit_length() + 1) // 2)
    while True:
        y = (x + n // x) // 2
        if y >= x:
            return x
        x = y


def _digits(text):
    if len(text) == 0 or not text.isdigit():
        raise 'ValueError: invalid literal for Decimal: ' + repr(text)
    return int(text)


def _parse(text):
    text = text.strip()
    if len(text) == 0:
        raise 'ValueError: invalid literal for Decimal: ' + repr(text)
    sign = 0
    if text[0] == '+':
        text = text[1:]
    elif text[0] == '-':
        sign = 1
        text = text[1:]
    upper = text.upper()
    if upper == 'INF' or upper == 'INFINITY':
        return sign, False, True, 0, 0
    if upper == 'NAN' or upper.startswith('NAN'):
        return sign, True, False, 0, 0
    if upper.startswith('SNAN'):
        return sign, True, False, 0, 0
    exponent = 0
    if 'E' in upper:
        at = upper.index('E')
        exponent = int(text[at + 1:])
        text = text[:at]
    if '.' in text:
        whole, frac = text.split('.', 1)
    else:
        whole, frac = text, ''
    digits = whole + frac
    if digits == '':
        raise 'ValueError: invalid literal for Decimal: ' + repr(text)
    coefficient = _digits(digits)
    exponent -= len(frac)
    return sign, False, False, coefficient, exponent


class Decimal:
    def __init__(self, value='0', context=None):
        if isinstance(value, Decimal):
            self._sign = value._sign
            self._nan = value._nan
            self._inf = value._inf
            self._int = value._int
            self._exp = value._exp
            return
        if isinstance(value, str):
            sign, nan, inf, coefficient, exponent = _parse(value)
            self._sign, self._nan, self._inf = sign, nan, inf
            self._int, self._exp = coefficient, exponent
            return
        if isinstance(value, bool):
            value = int(value)
        if isinstance(value, int):
            self._sign = 1 if value < 0 else 0
            self._nan = False
            self._inf = False
            self._int = -value if value < 0 else value
            self._exp = 0
            return
        if isinstance(value, float):
            if value != value:
                self._sign = 1 if math.signbit(value) else 0
                self._nan = True
                self._inf = False
                self._int = 0
                self._exp = 0
                return
            if math.isinf(value):
                self._sign = 1 if value < 0 else 0
                self._nan = False
                self._inf = True
                self._int = 0
                self._exp = 0
                return
            n, d = value.as_integer_ratio()
            self._sign = 1 if n < 0 else 0
            n = -n if n < 0 else n
            twos = 0
            while d > 1:
                d //= 2
                twos += 1
            self._int = n * (5 ** twos)
            self._exp = -twos
            self._nan = False
            self._inf = False
            return
        raise 'TypeError: conversion from ' + repr(type(value).__name__) + ' to Decimal is not supported'

    def _finite(sign, coefficient, exponent):
        made = Decimal(0)
        made._sign = 1 if sign else 0
        made._nan = False
        made._inf = False
        made._int = coefficient
        made._exp = exponent
        return made

    def is_nan(self):
        return self._nan

    def is_infinite(self):
        return self._inf

    def is_finite(self):
        return not self._nan and not self._inf

    def is_zero(self):
        return self.is_finite() and self._int == 0

    def is_signed(self):
        return self._sign == 1

    def as_tuple(self):
        if self._nan:
            digits = tuple(int(c) for c in str(self._int)) if self._int else ()
            return (self._sign, digits, 'n')
        if self._inf:
            return (self._sign, (), 'F')
        digits = str(self._int)
        return (self._sign, tuple(int(c) for c in digits), self._exp)

    def __str__(self):
        if self._nan:
            return ('-' if self._sign else '') + 'NaN'
        if self._inf:
            return ('-' if self._sign else '') + 'Infinity'
        digits = str(self._int)
        sign = '-' if self._sign else ''
        exponent = self._exp
        adjusted = exponent + len(digits) - 1
        if exponent <= 0 and adjusted >= -6:
            if exponent == 0:
                return sign + digits
            if -exponent < len(digits):
                point = len(digits) + exponent
                return sign + digits[:point] + '.' + digits[point:]
            return sign + '0.' + ('0' * (-exponent - len(digits))) + digits
        # Scientific notation for anything else, one digit ahead of the
        # point, the way a decimal out of ordinary range is written.
        if len(digits) == 1:
            mantissa = digits
        else:
            mantissa = digits[0] + '.' + digits[1:]
        return sign + mantissa + 'E' + ('+' if adjusted >= 0 else '') + str(adjusted)

    def __repr__(self):
        return "Decimal('" + self.__str__() + "')"

    def __float__(self):
        if self._nan:
            return float('nan') if not self._sign else -float('nan')
        if self._inf:
            return float('-inf') if self._sign else float('inf')
        # A ratio of two whole numbers, divided by the language's own
        # working rather than by shifting a whole coefficient upward
        # first, which would answer for its own magnitude before ever
        # reaching a division that knows what to make of it.
        n, d = self.as_integer_ratio()
        value = n / d
        if value == float('inf') or value == float('-inf'):
            raise 'OverflowError: cannot convert Decimal to float'
        return value

    def __int__(self):
        if self._nan or self._inf:
            raise 'ValueError: cannot convert NaN or Infinity to integer'
        whole = self._int * (10 ** self._exp) if self._exp >= 0 else self._int // (10 ** (-self._exp))
        return -whole if self._sign else whole

    def __bool__(self):
        return self._nan or self._inf or self._int != 0

    def __neg__(self):
        return Decimal._finite(0 if self._sign else 1, self._int, self._exp) if self.is_finite() else _flipped(self)

    def __pos__(self):
        return Decimal(self)

    def __abs__(self):
        return Decimal._finite(False, self._int, self._exp) if self.is_finite() else _nonneg(self)

    def _add(self, other, negate_other):
        other = _coerced(other)
        if self._nan or other._nan:
            return _nan_of(self, other)
        other_sign = (not other._sign) if negate_other else other._sign
        if self._inf or other._inf:
            if self._inf and other._inf and self._sign != other_sign:
                raise 'ValueError: -Infinity + Infinity in Decimal addition'
            return _infinite(self._sign if self._inf else other_sign)
        exponent = self._exp if self._exp < other._exp else other._exp
        a = self._int * (10 ** (self._exp - exponent))
        b = other._int * (10 ** (other._exp - exponent))
        a = -a if self._sign else a
        b = -b if other_sign else b
        total = a + b
        sign = total < 0
        return Decimal._finite(sign, -total if sign else total, exponent)

    def __add__(self, other):
        return self._add(other, False)

    def __radd__(self, other):
        return self._add(other, False)

    def __sub__(self, other):
        return self._add(other, True)

    def __rsub__(self, other):
        return _coerced(other)._add(self, True)

    def __mul__(self, other):
        other = _coerced(other)
        if self._nan or other._nan:
            return _nan_of(self, other)
        sign = self._sign != other._sign
        if self._inf or other._inf:
            if (self._inf and other.is_zero()) or (other._inf and self.is_zero()):
                raise 'ValueError: 0 * Infinity in Decimal multiplication'
            return _infinite(sign)
        return Decimal._finite(sign, self._int * other._int, self._exp + other._exp)

    def __rmul__(self, other):
        return self.__mul__(other)

    def __pow__(self, other):
        if not isinstance(other, int):
            raise "TypeError: unsupported operand type(s) for ** or pow(): 'Decimal' and '" + type(other).__name__ + "'"
        if other < 0:
            raise 'NotImplementedError: negative Decimal powers are not supported'
        result = Decimal(1)
        base = Decimal(self)
        e = other
        while e > 0:
            if e & 1:
                result = result * base
            e >>= 1
            if e > 0:
                base = base * base
        return result

    def sqrt(self, context=None):
        if self._nan:
            return self
        if self._inf:
            if self._sign:
                raise 'ValueError: sqrt of a negative value'
            return self
        if self._sign and self._int != 0:
            raise 'ValueError: sqrt of a negative value'
        if self._int == 0:
            return Decimal._finite(self._sign, 0, self._exp // 2)
        # Rather than the context's own count of figures, enough are
        # kept that nothing built from this root -- a binary real
        # included -- ever finds fewer of them correct than it needs.
        guard = 40
        exponent = self._exp
        coefficient = self._int
        if exponent % 2 != 0:
            coefficient *= 10
            exponent -= 1
        scaled = coefficient * (10 ** (2 * guard))
        root = _isqrt(scaled)
        return Decimal._finite(False, root, exponent // 2 - guard)

    def _cmp(self, other):
        other = _coerced(other)
        if self._nan or other._nan:
            return None
        if self._inf or other._inf:
            left = float('inf') * (-1 if self._sign else 1) if self._inf else float(self)
            right = float('inf') * (-1 if other._sign else 1) if other._inf else float(other)
            return -1 if left < right else (1 if left > right else 0)
        exponent = self._exp if self._exp < other._exp else other._exp
        a = self._int * (10 ** (self._exp - exponent))
        b = other._int * (10 ** (other._exp - exponent))
        a = -a if self._sign else a
        b = -b if other._sign else b
        return -1 if a < b else (1 if a > b else 0)

    def __eq__(self, other):
        if not isinstance(other, Decimal) and not isinstance(other, int) and not isinstance(other, bool):
            return NotImplemented
        return self._cmp(other) == 0

    def __ne__(self, other):
        result = self.__eq__(other)
        if result is NotImplemented:
            return result
        return not result

    def __lt__(self, other):
        c = self._cmp(other)
        if c is None:
            return False
        return c < 0

    def __le__(self, other):
        c = self._cmp(other)
        if c is None:
            return False
        return c <= 0

    def __gt__(self, other):
        c = self._cmp(other)
        if c is None:
            return False
        return c > 0

    def __ge__(self, other):
        c = self._cmp(other)
        if c is None:
            return False
        return c >= 0

    def as_integer_ratio(self):
        if self._nan:
            raise 'ValueError: cannot convert NaN to integer ratio'
        if self._inf:
            raise 'OverflowError: cannot convert Infinity to integer ratio'
        if self._int == 0:
            return 0, 1
        if self._exp >= 0:
            n, d = self._int * (10 ** self._exp), 1
        else:
            n, d = self._int, 10 ** (-self._exp)
            g = _gcd_int(n, d)
            n, d = n // g, d // g
        return (-n if self._sign else n), d

    def __hash__(self):
        if self._nan:
            return hash(float('nan'))
        if self._inf:
            return hash(float('-inf') if self._sign else float('inf'))
        return hash(float(self)) if self._exp < 0 else hash(self.__int__())


def _infinite(sign):
    made = Decimal(0)
    made._sign = 1 if sign else 0
    made._nan = False
    made._inf = True
    made._int = 0
    made._exp = 0
    return made


def _flipped(value):
    made = Decimal(value)
    made._sign = 0 if made._sign else 1
    return made


def _nonneg(value):
    made = Decimal(value)
    made._sign = 0
    return made


def _nan_of(a, b):
    source = a if a._nan else b
    made = Decimal(0)
    made._sign = source._sign
    made._nan = True
    made._inf = False
    made._int = 0
    made._exp = 0
    return made


def _coerced(value):
    if isinstance(value, Decimal):
        return value
    if isinstance(value, bool):
        return Decimal(int(value))
    if isinstance(value, int):
        return Decimal(value)
    raise "TypeError: unsupported operand type(s): 'Decimal' and '" + type(value).__name__ + "'"
