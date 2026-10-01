# A decimal held exactly, the way its text is written: a sign, a whole
# number of digits and the power of ten they stand at, with infinity
# and the value nothing is equal to kept apart as marks of their own.
# Arithmetic here is carried out exactly; only a root, which no decimal
# answers exactly, is rounded, and only then to as many figures as the
# working needs rather than to a context's own count -- more than
# enough for anything this language converts such a root through, a
# binary real included, and never fewer.

import math


ROUND_CEILING = 'ROUND_CEILING'
ROUND_DOWN = 'ROUND_DOWN'
ROUND_FLOOR = 'ROUND_FLOOR'
ROUND_HALF_DOWN = 'ROUND_HALF_DOWN'
ROUND_HALF_EVEN = 'ROUND_HALF_EVEN'
ROUND_HALF_UP = 'ROUND_HALF_UP'
ROUND_UP = 'ROUND_UP'
ROUND_05UP = 'ROUND_05UP'


class DecimalException(ArithmeticError):
    """The base of everything a decimal working can go wrong by."""


class InvalidOperation(DecimalException):
    """An operation the decimal scheme gives no answer to."""


class DivisionByZero(DecimalException, ZeroDivisionError):
    """A division by a value nothing is left of."""


class Overflow(DecimalException):
    """A result past every number the context's range holds."""


class Underflow(DecimalException):
    """A result under every number the context's range holds."""


class Clamped(DecimalException):
    """An exponent the context's range had to press in."""


class Context:
    def __init__(self, prec=28, rounding=None, Emin=None, Emax=None, capitals=1, clamp=0, flags=None, traps=None):
        self.prec = prec
        self.rounding = rounding if rounding is not None else ROUND_HALF_EVEN
        self.Emin = Emin if Emin is not None else -999999
        self.Emax = Emax if Emax is not None else 999999
        self.capitals = capitals
        self.clamp = clamp
        self.traps = traps if traps is not None else {InvalidOperation, DivisionByZero, Overflow}

    def copy(self):
        return Context(self.prec, self.rounding, self.Emin, self.Emax, self.capitals, self.clamp, traps=set(self.traps))

    def __repr__(self):
        return 'Context(prec=' + repr(self.prec) + ', rounding=' + repr(self.rounding) + ')'


# The three settings the specification offers ready-made: the one
# every run starts under, and the two a run may borrow for a while.
DefaultContext = Context(prec=28, rounding=ROUND_HALF_EVEN,
                         traps={DivisionByZero, Overflow, InvalidOperation},
                         Emax=999999, Emin=-999999, capitals=1, clamp=0)
BasicContext = Context(prec=9, rounding=ROUND_HALF_UP,
                       traps={DivisionByZero, Overflow, InvalidOperation, Clamped, Underflow},
                       Emax=999999, Emin=-999999, capitals=1, clamp=0)
ExtendedContext = Context(prec=9, rounding=ROUND_HALF_EVEN, traps=set(),
                          Emax=999999, Emin=-999999, capitals=1, clamp=0)

_context = DefaultContext.copy()


def _signal(kind, message):
    # A trapped signal stops the work as the exception of its own kind;
    # one no trap watches answers as the quiet value nothing equals.
    if kind in getcontext().traps:
        raise kind(message)
    made = Decimal(0)
    made._nan = True
    made._inf = False
    return made


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
        return sign, 's', False, 0, 0
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
            self._snan = value._snan
            self._inf = value._inf
            self._int = value._int
            self._exp = value._exp
            return
        if isinstance(value, str):
            sign, nan, inf, coefficient, exponent = _parse(value)
            self._snan = nan == 's'
            self._nan = bool(nan)
            self._sign, self._inf = sign, inf
            self._int, self._exp = coefficient, exponent
            return
        self._snan = False
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
        made._snan = False
        made._inf = False
        made._int = coefficient
        made._exp = exponent
        return made

    def is_nan(self):
        return self._nan

    def is_snan(self):
        return self._nan and self._snan

    def is_qnan(self):
        return self._nan and not self._snan

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
            return (self._sign, digits, 'N' if self._snan else 'n')
        if self._inf:
            return (self._sign, (), 'F')
        digits = str(self._int)
        return (self._sign, tuple(int(c) for c in digits), self._exp)

    def __str__(self):
        if self._nan:
            return ('-' if self._sign else '') + ('sNaN' if self._snan else 'NaN')
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

    def __format__(self, spec):
        # Work with decimal digits throughout, including halfway rounding.
        # Converting through a binary float would lose both precision and
        # trailing zeros before the requested presentation is known.
        import re
        match = re.fullmatch(r'(?:(.)([<>=^])|([<>=^]))?([+ -])?(0)?([0-9]*)(,)?(?:\.([0-9]+))?([eEfFgG%])?', spec)
        if match is None:
            raise ValueError('invalid format string')
        fill, alignment, plain_align, sign, zero, width, grouping, precision, kind = match.groups()
        alignment = alignment or plain_align or '>'
        fill = fill or ' '
        if zero:
            fill, alignment = '0', '='
        width = int(width) if width else 0
        precision = int(precision) if precision is not None else None
        kind = kind or ('G' if getcontext().capitals else 'g')
        coefficient, exponent = self._int, self._exp
        if kind == '%':
            exponent += 2
        if self._nan or self._inf:
            body = 'NaN' if self._nan else 'Infinity'
            if zero:
                fill, alignment = ' ', '>'
        else:
            figures = len(str(coefficient))
            adjusted = exponent + figures - 1
            if precision is None:
                target = exponent
            elif kind in 'fF%':
                target = -precision
            elif kind in 'eE':
                target = adjusted - precision
            else:
                target = adjusted - max(precision, 1) + 1
            if target > exponent:
                divisor = 10 ** (target - exponent)
                kept, rest = divmod(coefficient, divisor)
                rounding = getcontext().rounding
                increment = False
                if rounding == 'ROUND_UP':
                    increment = rest != 0
                elif rounding == 'ROUND_CEILING':
                    increment = rest != 0 and not self._sign
                elif rounding == 'ROUND_FLOOR':
                    increment = rest != 0 and self._sign
                elif rounding == 'ROUND_05UP':
                    increment = rest != 0 and kept % 10 in (0, 5)
                elif rounding != 'ROUND_DOWN':
                    increment = rest * 2 > divisor or (rest * 2 == divisor and (rounding == 'ROUND_HALF_UP' or (rounding == 'ROUND_HALF_EVEN' and kept % 2 == 1)))
                coefficient, exponent = kept + int(increment), target
            elif target < exponent and kind in 'eEfF%':
                coefficient *= 10 ** (exponent - target)
                exponent = target
            if precision is not None and kind in 'gGeE':
                limit = max(precision, 1) if kind in 'gG' else precision + 1
                if len(str(coefficient)) > limit:
                    coefficient //= 10
                    exponent += 1
            digits = str(coefficient)
            adjusted = exponent + len(digits) - 1
            if kind in 'eE' or kind in 'gG' and (exponent > 0 or adjusted < -6):
                if coefficient == 0:
                    adjusted = 0 if precision is not None else exponent
                if precision is not None and kind in 'eE':
                    digits = (digits + '0' * precision)[:precision + 1]
                body = digits[0] + ('.' + digits[1:] if len(digits) > 1 else '')
                body += ('E' if kind in 'EG' else 'e') + ('+' if adjusted >= 0 else '-') + str(abs(adjusted))
            else:
                point = len(digits) + exponent
                if point <= 0:
                    body = '0.' + '0' * (-point) + digits
                elif point < len(digits):
                    body = digits[:point] + '.' + digits[point:]
                else:
                    body = digits + '0' * exponent
            if grouping:
                parts = body.split('.', 1)
                whole = parts[0]
                groups = []
                while len(whole) > 3:
                    groups.insert(0, whole[-3:])
                    whole = whole[:-3]
                groups.insert(0, whole)
                body = ','.join(groups) + ('.' + parts[1] if len(parts) > 1 else '')
        prefix = '-' if self._sign else ('+' if sign == '+' else (' ' if sign == ' ' else ''))
        if kind == '%':
            body += '%'
        padding = max(0, width - len(prefix) - len(body))
        if alignment == '=':
            return prefix + fill * padding + body
        result = prefix + body
        if alignment == '<':
            return result + fill * padding
        if alignment == '^':
            left = padding // 2
            return fill * left + result + fill * (padding - left)
        return fill * padding + result

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
        # The reference's plus applies the context to what it answers:
        # a value already within the context's figures is given back
        # as it stands, and one beyond them is rounded into them.
        return _context_round(self, getcontext())

    def __abs__(self):
        return Decimal._finite(False, self._int, self._exp) if self.is_finite() else _nonneg(self)

    def _rounded_to(self, exponent):
        # The coefficient as it stands at the given exponent, a halfway
        # digit going to the even neighbour, as the reference rounds
        # its decimals.
        if self._exp >= exponent:
            return self._int * (10 ** (self._exp - exponent))
        divisor = 10 ** (exponent - self._exp)
        whole, rest = divmod(self._int, divisor)
        twice = rest * 2
        if twice > divisor or (twice == divisor and whole % 2 == 1):
            whole += 1
        return whole

    def __round__(self, ndigits=None):
        if ndigits is None:
            # Rounded to a whole number the answer is a plain int, so
            # round(d) and round(d, None) answer alike.
            if self._nan:
                raise 'ValueError: cannot convert float NaN to integer'
            if self._inf:
                raise 'OverflowError: cannot convert float infinity to integer'
            whole = self._rounded_to(0)
            return -whole if self._sign else whole
        if self._nan or self._inf:
            return Decimal(self)
        return Decimal._finite(self._sign, self._rounded_to(-ndigits), -ndigits)

    def _add(self, other, negate_other):
        other = _coerced(other)
        if self._nan or other._nan:
            return _nan_of(self, other)
        other_sign = (not other._sign) if negate_other else other._sign
        if self._inf or other._inf:
            if self._inf and other._inf and self._sign != other_sign:
                return _signal(InvalidOperation, '-Infinity + Infinity')
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
                return _signal(InvalidOperation, '0 * Infinity')
            return _infinite(sign)
        return Decimal._finite(sign, self._int * other._int, self._exp + other._exp)

    def __rmul__(self, other):
        return self.__mul__(other)

    def __pow__(self, other, modulo=None):
        if modulo is not None:
            raise 'NotImplementedError: modular Decimal powers are not supported'
        if isinstance(other, Decimal):
            if other._snan or self._snan:
                return _signal(InvalidOperation, 'a power met a signaling NaN')
            if other._nan or self._nan:
                return Decimal(self) if self._nan else Decimal(other)
            if other._inf:
                if self._inf or self._int == 0:
                    # An infinity to a positive power is itself, and a
                    # zero to one is zero; the other ways round they
                    # swap, and a negative power of nothing has no
                    # answer at all.
                    if other._sign:
                        return Decimal._finite(False, 0, 0) if self._int == 0 else Decimal._finite(False, 0, 0)
                    return self if self._inf else Decimal(self)
                adjusted = len(str(self._int)) + self._exp - 1
                if adjusted == 0 and self._int == 1:
                    return _signal(InvalidOperation, 'one to an infinite power')
                grows = adjusted > 0
                if grows != other._sign:
                    return _infinite(False)
                return Decimal._finite(False, 0, 0)
            if other._exp >= 0:
                if other._sign:
                    raise 'NotImplementedError: negative Decimal powers are not supported'
                whole = other._int * 10 ** other._exp
                return self._whole_power(whole)
            # A fractional power is asked only of a positive base, as
            # the reference asks it: nothing else has one answer.
            if self._inf:
                if other._sign:
                    return Decimal._finite(False, 0, 0)
                if self._sign:
                    return _signal(InvalidOperation, 'a negative base to a fractional power')
                return self
            if self._int == 0:
                if other._sign:
                    return _signal(InvalidOperation, 'zero to a negative power')
                return Decimal._finite(False, 0, 0)
            if self._sign:
                return _signal(InvalidOperation, 'a negative base to a fractional power')
            return (other * self.ln()).exp()
        if isinstance(other, bool):
            other = int(other)
        if isinstance(other, int):
            if other < 0:
                raise 'NotImplementedError: negative Decimal powers are not supported'
            return self._whole_power(other)
        raise "TypeError: unsupported operand type(s) for ** or pow(): 'Decimal' and '" + type(other).__name__ + "'"

    def _whole_power(self, whole):
        result = Decimal(1)
        base = Decimal(self)
        e = whole
        while e > 0:
            if e & 1:
                result = result * base
            e >>= 1
            if e > 0:
                base = base * base
        return result

    def __truediv__(self, other):
        return self._divide(other)

    def __rtruediv__(self, other):
        return _coerced(other)._divide(self)

    def _divide(self, other):
        other = _coerced(other)
        if self._nan or other._nan:
            return _nan_of(self, other)
        sign = self._sign != other._sign
        if self._inf:
            if other._inf:
                return _signal(InvalidOperation, 'Infinity / Infinity')
            if other._int == 0:
                return _signal(DivisionByZero, 'division by zero')
            return _infinite(sign)
        if other._inf:
            return Decimal._finite(sign, 0, 0)
        if other._int == 0:
            if self._int == 0:
                return _signal(InvalidOperation, '0 / 0')
            return _signal(DivisionByZero, 'division by zero')
        if self._int == 0:
            return Decimal._finite(sign, 0, 0)
        context = getcontext()
        prec = context.prec
        A, ea = self._int, self._exp
        B, eb = other._int, other._exp
        ideal = ea - eb
        # An exact answer first: where the divisor, once what the two
        # share is taken away, is a product of twos and fives alone,
        # the quotient is a whole number of digits at some place.
        g = _gcd_int(A, B)
        a, b = A // g, B // g
        twos = 0
        while b % 2 == 0:
            b //= 2
            twos += 1
        fives = 0
        while b % 5 == 0:
            b //= 5
            fives += 1
        if b == 1:
            raise_exponent = twos if twos > fives else fives
            coefficient = a * 10 ** raise_exponent // ((2 ** twos) * (5 ** fives))
            exponent = ideal - raise_exponent
            while coefficient % 10 == 0 and exponent < ideal:
                coefficient //= 10
                exponent += 1
            if len(str(coefficient)) <= prec:
                return Decimal._finite(sign, coefficient, exponent)
        # Otherwise, or where exactness outgrows the context, the
        # quotient is taken to one figure beyond the context's count
        # and rounded into it by the context's own rule.
        num, den, shift = A, B, 0
        while num // den < 10 ** prec:
            num *= 10
            shift -= 1
        while num // den >= 10 ** (prec + 1):
            den *= 10
            shift += 1
        whole, rest = divmod(num, den)
        kept = whole // 10
        dropped = whole % 10
        rounding = context.rounding
        upward = False
        if rounding == ROUND_HALF_EVEN:
            upward = dropped > 5 or (dropped == 5 and (rest != 0 or kept % 2 == 1))
        elif rounding == ROUND_HALF_UP:
            upward = dropped > 5 or (dropped == 5 and rest != 0) or dropped == 5
        elif rounding == ROUND_HALF_DOWN:
            upward = dropped > 5 or (dropped == 5 and rest != 0)
        elif rounding == ROUND_UP:
            upward = dropped != 0 or rest != 0
        elif rounding == ROUND_05UP:
            upward = (dropped != 0 or rest != 0) and kept % 5 == 0
        elif rounding == ROUND_CEILING:
            upward = dropped != 0 or rest != 0
        elif rounding == ROUND_FLOOR:
            upward = False
        else:
            upward = dropped > 5 or (dropped == 5 and (rest != 0 or kept % 2 == 1))
        if upward:
            kept += 1
        if kept == 10 ** prec:
            kept //= 10
            shift += 1
        return Decimal._finite(sign, kept, shift + ideal + 1)

    def __rpow__(self, other):
        return _coerced(other).__pow__(self)

    def sqrt(self, context=None):
        context = context if context is not None else getcontext()
        if self._snan:
            return _signal(InvalidOperation, 'sqrt of a signaling NaN')
        if self._nan:
            return self
        if self._inf:
            if self._sign:
                return _signal(InvalidOperation, 'sqrt of a negative value')
            return self
        if self._sign and self._int != 0:
            return _signal(InvalidOperation, 'sqrt of a negative value')
        if self._int == 0:
            return Decimal._finite(self._sign, 0, self._exp // 2)
        # Enough figures are taken that rounding to the context's own
        # count afterwards cannot disturb them: a span of guarding
        # digits beyond whatever the context asks for, never fewer than
        # a binary real would need of a root.
        guard = 25 if context.prec < 25 else context.prec + 25
        exponent = self._exp
        coefficient = self._int
        if exponent % 2 != 0:
            coefficient *= 10
            exponent -= 1
        scaled = coefficient * (10 ** (2 * guard))
        root = _isqrt(scaled)
        made = Decimal._finite(False, root, exponent // 2 - guard)
        # A root exact at fewer figures keeps no more than it needs:
        # trailing zeros fall away down to the places the root of the
        # value would naturally stand at.
        natural = (exponent // 2 - guard) + len(str(root))
        while made._int != 0 and made._int % 10 == 0 and made._exp < exponent // 2:
            made = Decimal._finite(False, made._int // 10, made._exp + 1)
        del natural
        return _context_round(made, context)

    def next_plus(self, context=None):
        context = context if context is not None else getcontext()
        return self._neighbour(context, 1)

    def next_minus(self, context=None):
        context = context if context is not None else getcontext()
        return self._neighbour(context, -1)

    def _neighbour(self, context, step):
        # The nearest value the context can hold on the one side of
        # this one: the coefficient is first brought to the context's
        # own count of figures, then walked a single place of its last
        # figure in the step's direction.
        if self._nan or self._inf:
            return Decimal(self)
        coefficient, exponent = self._int, self._exp
        digits = len(str(coefficient))
        if digits < context.prec:
            coefficient *= 10 ** (context.prec - digits)
            exponent -= context.prec - digits
        signed = -coefficient if self._sign else coefficient
        signed += step
        if signed == 0:
            return Decimal._finite(step < 0, 0, exponent)
        sign = signed < 0
        magnitude = -signed if sign else signed
        digits = len(str(magnitude))
        if digits > context.prec:
            magnitude //= 10
            exponent += 1
        elif digits < context.prec:
            magnitude = magnitude * 10 + (9 if step < 0 else 0)
            exponent -= 1
        return Decimal._finite(sign, magnitude, exponent)

    def ln(self, context=None):
        """The natural logarithm, rounded to the context's figures."""
        context = context if context is not None else getcontext()
        if self._snan:
            return _signal(InvalidOperation, 'ln of a signaling NaN')
        if self._nan:
            return Decimal(self)
        if self._inf:
            return self if not self._sign else _signal(InvalidOperation, 'ln of a negative value')
        if self._sign or self._int == 0:
            return _signal(InvalidOperation, 'ln of a non-positive value')
        working = context.prec + 15
        scale = 10 ** working
        coefficient, exponent = self._int, self._exp
        limit = working + 20
        # A coefficient of many thousands of digits is brought down to
        # its leading figures in one division, the size of it reckoned
        # from its own width first, so that neither a writing of all of
        # it nor a walk of thousands of places is ever asked for.
        shifted = 0
        width = (coefficient.bit_length() * 30103) // 100000
        if width > limit:
            shifted = width - limit + 1
            coefficient //= 10 ** shifted
        text = str(coefficient)
        lead = text if len(text) <= limit else text[:limit]
        lead_places = len(lead) - 1
        # ln(m * 10^lead_places) = ln(m) + lead_places * ln(10), with m
        # in [1, 10); the digits past the lead shift the place, their
        # own share of the answer far beneath the figures kept.
        places = len(text) - 1 + shifted
        mantissa_log = _ln_ratio(int(lead), 10 ** lead_places, scale)
        ten_log = _ln_ratio(10, 1, scale)
        total = mantissa_log + (places + exponent) * ten_log
        made = _scaled_to_decimal(total, scale)
        return _context_round(made, context)

    def exp(self, context=None):
        """The exponential, rounded to the context's figures."""
        context = context if context is not None else getcontext()
        if self._snan:
            return _signal(InvalidOperation, 'exp of a signaling NaN')
        if self._nan:
            return Decimal(self)
        if self._inf:
            if self._sign:
                return Decimal._finite(False, 0, 0)
            return self
        working = context.prec + 15
        scale = 10 ** working
        value = _exp_scaled(self._signed_int(), 10 ** (-self._exp) if self._exp < 0 else 1, scale)
        made = _scaled_to_decimal(value, scale)
        return _context_round(made, context)

    def _signed_int(self):
        return -self._int if self._sign else self._int

    def _cmp(self, other):
        other = Decimal(other) if isinstance(other, float) else _coerced(other)
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
        if isinstance(other, complex):
            if other.imag != 0:
                return False
            other = other.real
        if not isinstance(other, (Decimal, int, float)) and not isinstance(other, bool):
            return NotImplemented
        return self._cmp(other) == 0

    def __ne__(self, other):
        result = self.__eq__(other)
        if result is NotImplemented:
            return result
        return not result

    def __lt__(self, other):
        try:
            c = self._cmp(other)
        except TypeError:
            # Fraction and other Rational-like types know how to compare
            # with Decimal via the reflected comparison operator.
            return NotImplemented
        if c is None:
            return False
        return c < 0

    def __le__(self, other):
        try:
            c = self._cmp(other)
        except TypeError:
            return NotImplemented
        if c is None:
            return False
        return c <= 0

    def __gt__(self, other):
        try:
            c = self._cmp(other)
        except TypeError:
            return NotImplemented
        if c is None:
            return False
        return c > 0

    def __ge__(self, other):
        try:
            c = self._cmp(other)
        except TypeError:
            return NotImplemented
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
    # A signaling operand stops the working as an invalid operation
    # wherever a trap watches for it, and quiets into the answer
    # nothing equals where none does.
    if getattr(a, '_snan', False) or getattr(b, '_snan', False):
        return _signal(InvalidOperation, 'an operation met a signaling NaN')
    source = a if a._nan else b
    made = Decimal(0)
    made._sign = source._sign
    made._nan = True
    made._snan = False
    made._inf = False
    made._int = 0
    made._exp = 0
    return made


def _context_round(value, context):
    # What the context leaves of a value: one already within its count
    # of figures is given back untouched, and one beyond them is
    # rounded into them by the context's own rule. Nothing is padded.
    if value._nan or value._inf:
        return Decimal(value)
    coefficient, exponent = value._int, value._exp
    digits = len(str(coefficient))
    if digits <= context.prec:
        return Decimal(value)
    drop = digits - context.prec
    kept = coefficient // 10 ** drop
    rest = coefficient % 10 ** drop
    rounding = context.rounding
    upward = False
    if rounding == ROUND_HALF_EVEN:
        twice = rest * 2
        place = 10 ** drop
        upward = twice > place or (twice == place and kept % 2 == 1)
    elif rounding == ROUND_HALF_UP:
        upward = rest * 2 >= 10 ** drop
    elif rounding == ROUND_HALF_DOWN:
        upward = rest * 2 > 10 ** drop
    elif rounding == ROUND_UP:
        upward = rest != 0
    elif rounding == ROUND_05UP:
        upward = rest != 0 and kept % 5 == 0
    elif rounding == ROUND_CEILING:
        upward = rest != 0 and not value._sign
    elif rounding == ROUND_FLOOR:
        upward = rest != 0 and value._sign
    if upward:
        kept += 1
    if kept == 10 ** context.prec:
        kept //= 10
        exponent += 1 + drop
        return Decimal._finite(value._sign, kept, exponent)
    return Decimal._finite(value._sign, kept, exponent + drop)


def _scaled_to_decimal(scaled, scale):
    # A fixed-point whole, standing for scaled / scale, as a Decimal.
    if scaled == 0:
        return Decimal._finite(False, 0, 0)
    negative = scaled < 0
    magnitude = -scaled if negative else scaled
    exponent = 0
    while magnitude % 10 == 0 and magnitude != 0:
        magnitude //= 10
        exponent += 1
    places = len(str(scale)) - 1
    return Decimal._finite(negative, magnitude, exponent - places)


def _ln_ratio(numerator, denominator, scale):
    # ln(numerator / denominator) as a fixed-point whole at the scale,
    # for a positive ratio in [1, 10). Square roots bring the ratio
    # near one first -- each root halves the logarithm -- and the odd
    # series of the inverse hyperbolic tangent, whose terms fall away
    # fast once there, finds what is left. A ratio of one has no
    # logarithm to find.
    if numerator == denominator:
        return 0
    held = numerator * scale // denominator
    roots = 0
    while held > scale + scale // 5:
        held = _isqrt(held * scale)
        roots += 1
    p = held - scale
    if p == 0:
        return 0
    q = held + scale
    total = 0
    at = 1
    while True:
        term = p ** at * scale // (q ** at * at)
        if term == 0 and at > 1:
            break
        total += term
        at += 2
    return total * 2 ** (roots + 1)


def _exp_scaled(numerator, denominator, scale):
    # exp(numerator / denominator) as a fixed-point whole at the scale,
    # for a ratio brought small first, halved away and squared back.
    negative = numerator < 0
    n = -numerator if negative else numerator
    halves = 0
    while n >= denominator:
        denominator *= 2
        halves += 1
    while n * 100 >= denominator:
        denominator *= 2
        halves += 1
    # the Taylor run on the small ratio n / denominator
    total = scale
    term = scale
    at = 1
    while True:
        term = term * n // (denominator * at)
        if term == 0:
            break
        total += term
        at += 1
    for _ in range(halves):
        total = total * total // scale
    if negative:
        # exp(-x) = scale**2 // exp(x), at the scale
        total = scale * scale // total
    return total


def _coerced(value):
    if isinstance(value, Decimal):
        return value
    if isinstance(value, bool):
        return Decimal(int(value))
    if isinstance(value, int):
        return Decimal(value)
    raise "TypeError: unsupported operand type(s): 'Decimal' and '" + type(value).__name__ + "'"
