# Ratios are kept as two whole numbers with a positive denominator.
def _gcd(a, b):
    if a < 0:
        a = -a
    while b != 0:
        a, b = b, a % b
    return a

def _trim(text):
    while len(text) and text[0] in ' \t\n\r\v\f':
        text = text[1:]
    while len(text) and text[-1] in ' \t\n\r\v\f':
        text = text[:-1]
    return text

def _digits(text):
    if len(text) == 0:
        raise 'ValueError: Invalid literal for Fraction'
    return int(text)

def _parse(text):
    text = _trim(text)
    for at in range(len(text)):
        if text[at] == '/':
            return _digits(_trim(text[:at])), _digits(_trim(text[at + 1:]))
    exponent = 0
    for at in range(len(text)):
        if text[at] in 'eE':
            exponent = _digits(text[at + 1:])
            text = text[:at]
            break
    digits = ''
    places = 0
    point = False
    for char in text:
        if char == '.':
            if point:
                raise 'ValueError: Invalid literal for Fraction'
            point = True
        else:
            digits += char
            if point and char != '_':
                places += 1
    n = _digits(digits)
    scale = places - exponent
    if scale < 0:
        return n * 10 ** (-scale), 1
    return n, 10 ** scale

def _ratio(value):
    n = getattr(value, 'numerator', None)
    if n is not None:
        return n, value.denominator
    if str(value) == value:
        return _parse(value)
    if value != value:
        raise 'ValueError: cannot convert NaN to integer ratio'
    if value == float('inf') or value == float('-inf'):
        raise 'OverflowError: cannot convert Infinity to integer ratio'
    denominator = 1
    while value != int(value):
        value *= 2
        denominator *= 2
    return int(value), denominator

def _number(value):
    if str(value) == value:
        raise 'TypeError: unsupported operand type for Fraction'
    return Fraction(value)

class Fraction:
    def __init__(self, numerator=0, denominator=None):
        n, d = _ratio(numerator)
        if denominator is not None:
            if n != numerator or d != 1 or int(denominator) != denominator:
                raise 'TypeError: both arguments should be Rational instances'
            d = int(denominator)
        if d == 0:
            raise 'ZeroDivisionError: Fraction denominator is zero'
        if d < 0:
            n, d = -n, -d
        common = _gcd(n, d)
        self.numerator = n // common
        self.denominator = d // common

    def as_integer_ratio(self):
        return self.numerator, self.denominator

    def __str__(self):
        if self.denominator == 1:
            return str(self.numerator)
        return str(self.numerator) + '/' + str(self.denominator)

    def __repr__(self):
        return 'Fraction(' + str(self.numerator) + ', ' + str(self.denominator) + ')'

    def __float__(self):
        return self.numerator / self.denominator

    def __int__(self):
        if self.numerator < 0:
            return -((-self.numerator) // self.denominator)
        return self.numerator // self.denominator

    def __neg__(self):
        return Fraction(-self.numerator, self.denominator)

    def __pos__(self):
        return Fraction(self.numerator, self.denominator)

    def __abs__(self):
        if self.numerator < 0:
            return self.__neg__()
        return self.__pos__()

    def __add__(self, other):
        right = _number(other)
        return Fraction(self.numerator * right.denominator + right.numerator * self.denominator, self.denominator * right.denominator)

    def __radd__(self, other):
        return self.__add__(other)

    def __sub__(self, other):
        right = _number(other)
        return Fraction(self.numerator * right.denominator - right.numerator * self.denominator, self.denominator * right.denominator)

    def __rsub__(self, other):
        return _number(other).__sub__(self)

    def __mul__(self, other):
        right = _number(other)
        return Fraction(self.numerator * right.numerator, self.denominator * right.denominator)

    def __rmul__(self, other):
        return self.__mul__(other)

    def __truediv__(self, other):
        right = _number(other)
        return Fraction(self.numerator * right.denominator, self.denominator * right.numerator)

    def __rtruediv__(self, other):
        return _number(other).__truediv__(self)

    def __floordiv__(self, other):
        right = _number(other)
        return (self.numerator * right.denominator) // (self.denominator * right.numerator)

    def __rfloordiv__(self, other):
        return _number(other).__floordiv__(self)

    def __mod__(self, other):
        return self - self.__floordiv__(other) * _number(other)

    def __rmod__(self, other):
        return _number(other).__mod__(self)

    def __pow__(self, other):
        if other != int(other):
            raise 'NotImplementedError: non-integral Fraction powers are not supported'
        exponent = int(other)
        if exponent < 0:
            return Fraction(self.denominator ** (-exponent), self.numerator ** (-exponent))
        return Fraction(self.numerator ** exponent, self.denominator ** exponent)

    def __rpow__(self, other):
        if self.denominator != 1:
            raise 'NotImplementedError: non-integral Fraction powers are not supported'
        return other ** self.numerator

    def __eq__(self, other):
        if str(other) == other:
            return False
        right = _number(other)
        return self.numerator == right.numerator and self.denominator == right.denominator

    def __ne__(self, other):
        return not self.__eq__(other)

    def __lt__(self, other):
        right = _number(other)
        return self.numerator * right.denominator < right.numerator * self.denominator

    def __le__(self, other):
        right = _number(other)
        return self.numerator * right.denominator <= right.numerator * self.denominator

    def __gt__(self, other):
        right = _number(other)
        return self.numerator * right.denominator > right.numerator * self.denominator

    def __ge__(self, other):
        right = _number(other)
        return self.numerator * right.denominator >= right.numerator * self.denominator

    def limit_denominator(self, max_denominator=1000000):
        if max_denominator < 1:
            raise 'ValueError: max_denominator should be at least 1'
        if self.denominator <= max_denominator:
            return Fraction(self)
        p0, q0, p1, q1 = 0, 1, 1, 0
        n, d = self.numerator, self.denominator
        while True:
            a = n // d
            q2 = q0 + a * q1
            if q2 > max_denominator:
                break
            p0, q0, p1, q1 = p1, q1, p0 + a * p1, q2
            n, d = d, n - a * d
        k = (max_denominator - q0) // q1
        bound1 = Fraction(p0 + k * p1, q0 + k * q1)
        bound2 = Fraction(p1, q1)
        distance1 = (bound1 - self).__abs__()
        distance2 = (bound2 - self).__abs__()
        if distance2 <= distance1:
            return bound2
        return bound1
