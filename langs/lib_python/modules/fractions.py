# Ratios are kept as two whole numbers with a positive denominator.
from math import inf
import numbers
def _gcd(a, b):
    if a < 0:
        a = -a
    while b != 0:
        a, b = b, a % b
    return a

def _trim(text):
    while len(text):
        if text[0] not in ' \t\n\r\v\f':
            break
        text = text[1:]
    while len(text):
        if text[len(text) - 1] not in ' \t\n\r\v\f':
            break
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
            denominator = _trim(text[at + 1:])
            if len(denominator) == 0 or denominator[0] in '+-':
                raise 'ValueError: Invalid literal for Fraction'
            return _digits(_trim(text[:at])), _digits(denominator)
    exponent = 0
    for at in range(len(text)):
        if text[at] in 'eE':
            exponent = _digits(text[at + 1:])
            text = text[:at]
            break
    digits = ''
    places = 0
    point = False
    for at in range(len(text)):
        char = text[at]
        if char == '_':
            if at == 0 or at + 1 == len(text):
                raise 'ValueError: Invalid literal for Fraction'
            if text[at - 1] not in '0123456789' or text[at + 1] not in '0123456789':
                raise 'ValueError: Invalid literal for Fraction'
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

def _as_float(value):
    return __math('fdiv', value, 1.0)

def _ratio(value):
    if isinstance(value, bool):
        return int(value), 1
    n = getattr(value, 'numerator', None)
    if n is not None:
        return n, value.denominator
    if isinstance(value, str):
        return _parse(value)
    if isinstance(value, float):
        value = _as_float(value)
        if value != value:
            raise 'ValueError: cannot convert NaN to integer ratio'
        if value == inf or value == -inf:
            raise 'OverflowError: cannot convert Infinity to integer ratio'
        # A float's own as_integer_ratio() reads its bits directly, so
        # it costs the same whether the exponent is tiny or huge. The
        # doubling loop below is kept only for the duck-typed numerics
        # that reach this point without a float type of their own,
        # where a subnormal exponent would otherwise double a bignum
        # denominator on the order of a thousand times over.
        return value.as_integer_ratio()
    if value != value:
        raise 'ValueError: cannot convert NaN to integer ratio'
    if value == inf or value == -inf:
        raise 'OverflowError: cannot convert Infinity to integer ratio'
    denominator = 1
    while value != int(value):
        value *= 2
        denominator *= 2
    return int(value), denominator

def _number(value):
    if not isinstance(value, Fraction) and not isinstance(value, int) and not isinstance(value, float) and not isinstance(value, bool):
        raise 'TypeError: unsupported operand type for Fraction'
    return Fraction(value)

_DIGITS = '0123456789'
_ALIGNS = '<>=^'

def _spec_fill(spec):
    # The opening fill and alignment marks. A mark of alignment in second
    # place claims whatever stands before it, whatever that character is.
    if len(spec) > 1 and spec[1] in _ALIGNS:
        return spec[0], spec[1], 2
    if len(spec) > 0 and spec[0] in _ALIGNS:
        return None, spec[0], 1
    return None, None, 0

def _spec_run(spec, at):
    digits = ''
    while at < len(spec) and spec[at] in _DIGITS:
        digits += spec[at]
        at += 1
    return digits, at

def _spec_plain(spec):
    # The shape a ratio keeps for itself: fill, alignment, sign, the
    # alternate mark, a width and a grouping mark, and nothing else.
    # Nothing here asks for a rounded reading, so the ratio stays exact.
    fill, align, at = _spec_fill(spec)
    sign = ''
    if at < len(spec) and spec[at] in '+- ':
        sign = spec[at]
        at += 1
    alternate = False
    if at < len(spec) and spec[at] == '#':
        alternate = True
        at += 1
    width, at = _spec_run(spec, at)
    if len(width) > 1 and width[0] == '0':
        return None
    grouping = ''
    if at < len(spec) and spec[at] in ',_':
        grouping = spec[at]
        at += 1
    if at != len(spec):
        return None
    return [fill, align, sign, alternate, width, grouping]

def _spec_rounded(spec):
    # The shape a real number keeps, ending in one of the presentation
    # marks. A lone zero before the width is a width of its own; a zero
    # with a digit behind it asks for zero padding.
    fill, align, at = _spec_fill(spec)
    sign = ''
    if at < len(spec) and spec[at] in '+- ':
        sign = spec[at]
        at += 1
    no_minus_zero = False
    if at < len(spec) and spec[at] == 'z':
        no_minus_zero = True
        at += 1
    alternate = False
    if at < len(spec) and spec[at] == '#':
        alternate = True
        at += 1
    zeropad = False
    if at + 1 < len(spec) and spec[at] == '0' and spec[at + 1] in _DIGITS:
        zeropad = True
        at += 1
    width, at = _spec_run(spec, at)
    grouping = ''
    if at < len(spec) and spec[at] in ',_':
        grouping = spec[at]
        at += 1
    precision = ''
    fraction_mark = ''
    if at < len(spec) and spec[at] == '.':
        at += 1
        precision, at = _spec_run(spec, at)
        if at < len(spec) and spec[at] in ',_':
            fraction_mark = spec[at]
            at += 1
        elif len(precision) == 0:
            return None
    if at + 1 != len(spec) or spec[at] not in 'eEfFgG%':
        return None
    return [fill, align, sign, no_minus_zero, alternate, zeropad, width, grouping, precision, fraction_mark, spec[at]]

def _spec_pad(fill, align, width, sign, body):
    room = width - len(sign) - len(body)
    padding = ''
    if room > 0:
        padding = fill * room
    if align == '<':
        return sign + body + padding
    if align == '^':
        half = len(padding) // 2
        return padding[:half] + sign + body + padding[half:]
    if align == '=':
        return sign + padding + body
    return padding + sign + body

def _spec_group(digits, mark, from_left):
    # Digits parted into runs of three by a grouping mark, counted from
    # the left for the fractional side and from the right for the whole.
    if len(mark) == 0 or len(digits) == 0:
        return digits
    first = 3 if from_left else 1 + (len(digits) - 1) % 3
    grouped = digits[:first]
    at = first
    while at < len(digits):
        grouped += mark + digits[at:at + 3]
        at += 3
    return grouped

def _spec_amiss(spec, detail):
    raise 'ValueError: Invalid format specifier ' + repr(spec) + " for object of type 'Fraction'" + detail

class Fraction:
    def __init__(self, numerator=0, denominator=None):
        n, d = _ratio(numerator)
        if denominator is not None:
            if not (isinstance(numerator, int) or isinstance(numerator, bool) or isinstance(numerator, Fraction)) or not (isinstance(denominator, int) or isinstance(denominator, bool) or isinstance(denominator, Fraction)):
                raise 'TypeError: both arguments should be Rational instances'
            p, q = _ratio(denominator)
            n, d = n * q, d * p
        if d == 0:
            raise 'ZeroDivisionError: Fraction denominator is zero'
        if d < 0:
            n, d = -n, -d
        common = _gcd(n, d)
        self.numerator = n // common
        self.denominator = d // common

    def from_float(value):
        if not isinstance(value, int) and not isinstance(value, float):
            raise 'TypeError: Fraction.from_float() only takes floats and integers'
        return Fraction(value)

    def from_decimal(value):
        raise 'NotImplementedError: Decimal conversion is not supported'

    def as_integer_ratio(self):
        return self.numerator, self.denominator

    def __str__(self):
        if self.denominator == 1:
            return str(self.numerator)
        return str(self.numerator) + '/' + str(self.denominator)

    def __repr__(self):
        return 'Fraction(' + str(self.numerator) + ', ' + str(self.denominator) + ')'

    def __float__(self):
        value = _as_float(self.numerator / self.denominator)
        if value == inf or value == -inf:
            raise 'OverflowError: integer division result too large for a float'
        return value

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
        if isinstance(other, float):
            return _as_float(self.__float__() + _as_float(other))
        right = _number(other)
        return Fraction(self.numerator * right.denominator + right.numerator * self.denominator, self.denominator * right.denominator)

    def __radd__(self, other):
        return self.__add__(other)

    def __sub__(self, other):
        if isinstance(other, float):
            return _as_float(self.__float__() - _as_float(other))
        right = _number(other)
        return Fraction(self.numerator * right.denominator - right.numerator * self.denominator, self.denominator * right.denominator)

    def __rsub__(self, other):
        if isinstance(other, float):
            return _as_float(_as_float(other) - self.__float__())
        return _number(other).__sub__(self)

    def __mul__(self, other):
        if isinstance(other, float):
            return _as_float(self.__float__() * _as_float(other))
        right = _number(other)
        return Fraction(self.numerator * right.numerator, self.denominator * right.denominator)

    def __rmul__(self, other):
        return self.__mul__(other)

    def __truediv__(self, other):
        if isinstance(other, float):
            return _as_float(self.__float__() / _as_float(other))
        right = _number(other)
        return Fraction(self.numerator * right.denominator, self.denominator * right.numerator)

    def __rtruediv__(self, other):
        if isinstance(other, float):
            return _as_float(_as_float(other) / self.__float__())
        return _number(other).__truediv__(self)

    def __floordiv__(self, other):
        if isinstance(other, float):
            return _as_float(self.__float__() // _as_float(other))
        right = _number(other)
        return (self.numerator * right.denominator) // (self.denominator * right.numerator)

    def __rfloordiv__(self, other):
        if isinstance(other, float):
            return _as_float(_as_float(other) // self.__float__())
        return _number(other).__floordiv__(self)

    def __mod__(self, other):
        if isinstance(other, float):
            return _as_float(self.__float__() % _as_float(other))
        return self - self.__floordiv__(other) * _number(other)

    def __rmod__(self, other):
        if isinstance(other, float):
            return _as_float(_as_float(other) % self.__float__())
        return _number(other).__mod__(self)

    def __pow__(self, other):
        if isinstance(other, float):
            return _as_float(self.__float__() ** _as_float(other))
        if other != int(other):
            raise 'NotImplementedError: non-integral Fraction powers are not supported'
        exponent = int(other)
        if exponent < 0:
            return Fraction(self.denominator ** (-exponent), self.numerator ** (-exponent))
        return Fraction(self.numerator ** exponent, self.denominator ** exponent)

    def __rpow__(self, other):
        if self.denominator != 1:
            raise 'NotImplementedError: non-integral Fraction powers are not supported'
        if isinstance(other, float):
            return _as_float(_as_float(other) ** self.numerator)
        return other ** self.numerator

    def __eq__(self, other):
        if isinstance(other, float):
            if other != other:
                return False
            if other == inf:
                return False
            if other == -inf:
                return False
        if other is None or isinstance(other, str):
            return False
        # A complex with nothing imaginary is its real part; one with
        # an imaginary part equals no fraction.
        if isinstance(other, complex):
            if other.imag != 0:
                return False
            return self.__eq__(other.real)
        if not isinstance(other, Fraction) and not isinstance(other, int) and not isinstance(other, float) and not isinstance(other, bool):
            raise 'NotImplementedError: comparison with this Fraction operand is not supported'
        right = _number(other)
        return self.numerator == right.numerator and self.denominator == right.denominator

    def __ne__(self, other):
        return not self.__eq__(other)

    def __lt__(self, other):
        if isinstance(other, float):
            if other != other:
                return False
            if other == inf:
                return True
            if other == -inf:
                return False
        right = _number(other)
        return self.numerator * right.denominator < right.numerator * self.denominator

    def __le__(self, other):
        if isinstance(other, float):
            if other != other:
                return False
            if other == inf:
                return True
            if other == -inf:
                return False
        right = _number(other)
        return self.numerator * right.denominator <= right.numerator * self.denominator

    def __gt__(self, other):
        if isinstance(other, float):
            if other != other:
                return False
            if other == inf:
                return False
            if other == -inf:
                return True
        right = _number(other)
        return self.numerator * right.denominator > right.numerator * self.denominator

    def __ge__(self, other):
        if isinstance(other, float):
            if other != other:
                return False
            if other == inf:
                return False
            if other == -inf:
                return True
        right = _number(other)
        return self.numerator * right.denominator >= right.numerator * self.denominator

    def _round_to_exponent(self, exponent, no_minus_zero=False):
        # The nearest whole multiple of ten to the given power, ties
        # going to the even multiple. The answer is given as a sign, a
        # count of units and the power those units stand in.
        if exponent >= 0:
            top = self.numerator
            bottom = self.denominator * 10 ** exponent
        else:
            top = self.numerator * 10 ** (-exponent)
            bottom = self.denominator
        whole, left = divmod(top, bottom)
        if 2 * left > bottom or (2 * left == bottom and abs(whole) % 2 == 1):
            whole += 1
        if whole != 0:
            return whole < 0, abs(whole), exponent
        return self.numerator < 0 and not no_minus_zero, 0, exponent

    def _round_to_figures(self, figures):
        # The same rounding, but counted in significant figures. The
        # answer carries exactly that many figures unless it is zero.
        if self.numerator == 0:
            return False, 0, 1 - figures
        top = str(abs(self.numerator))
        bottom = str(self.denominator)
        reach = len(top) - len(bottom)
        if bottom <= top:
            reach += 1
        sign, units, exponent = self._round_to_exponent(reach - figures)
        if len(str(units)) == figures + 1:
            units //= 10
            exponent += 1
        return sign, units, exponent

    def _write_plain(self, shape):
        # A ratio written whole, with no rounding anywhere: the two sides
        # keep every digit they have and only the trimmings are honoured.
        fill = shape[0]
        if fill is None:
            fill = ' '
        align = shape[1]
        if align is None:
            align = '>'
        plus = shape[2]
        if plus == '-':
            plus = ''
        alternate = shape[3]
        width = 0
        if len(shape[4]):
            width = int(shape[4])
        grouping = shape[5]
        body = format(abs(self.numerator), grouping)
        if self.denominator > 1 or alternate:
            body += '/' + format(self.denominator, grouping)
        sign = plus
        if self.numerator < 0:
            sign = '-'
        return _spec_pad(fill, align, width, sign, body)

    def _write_rounded(self, shape):
        # A ratio written the way a real number is written, rounded to
        # the asked places or figures and then dressed the same way.
        fill = shape[0]
        if fill is None:
            fill = ' '
        align = shape[1]
        if align is None:
            align = '>'
        plus = shape[2]
        if plus == '-':
            plus = ''
        no_minus_zero = shape[3]
        alternate = shape[4]
        # Zero padding is asked for by the padding mark, and equally by a
        # fill of zero that is told to sit between the sign and the digits.
        zeropad = shape[5] or (fill == '0' and align == '=')
        width = 0
        if len(shape[6]):
            width = int(shape[6])
        grouping = shape[7]
        precision = 6
        if len(shape[8]):
            precision = int(shape[8])
        fraction_mark = shape[9]
        kind = shape[10]
        trim_zeros = kind in 'gG' and not alternate
        trim_point = not alternate
        marker = 'e'
        if kind in 'EFG':
            marker = 'E'
        if kind in 'fF%':
            exponent = -precision
            if kind == '%':
                exponent -= 2
            negative, units, exponent = self._round_to_exponent(exponent, no_minus_zero)
            scientific = False
            place = precision
        else:
            if kind in 'gG':
                figures = precision
                if figures < 1:
                    figures = 1
            else:
                figures = precision + 1
            negative, units, exponent = self._round_to_figures(figures)
            scientific = kind in 'eE' or exponent > 0 or exponent + figures <= -4
            place = -exponent
            if scientific:
                place = figures - 1
        if kind == '%':
            suffix = '%'
        elif scientific:
            reach = exponent + place
            shown = str(abs(reach))
            while len(shown) < 2:
                shown = '0' + shown
            suffix = marker + ('-' if reach < 0 else '+') + shown
        else:
            suffix = ''
        digits = str(units)
        while len(digits) < place + 1:
            digits = '0' + digits
        sign = plus
        if negative:
            sign = '-'
        leading = digits[:len(digits) - place]
        fraction = digits[len(digits) - place:]
        if trim_zeros:
            fraction = fraction.rstrip('0')
        point = '.'
        if trim_point and len(fraction) == 0:
            point = ''
        trailing = point + _spec_group(fraction, fraction_mark, True) + suffix
        if zeropad:
            room = width - len(sign) - len(trailing)
            # Grouping marks land in the padding too, so fewer digits
            # are needed to reach the asked width.
            if len(grouping) and room > 0:
                room = 3 * room // 4 + 1
            while len(leading) < room:
                leading = '0' + leading
        return _spec_pad(fill, align, width, sign, _spec_group(leading, grouping, False) + trailing)

    def __format__(self, format_spec):
        shape = _spec_plain(format_spec)
        if shape is not None:
            return self._write_plain(shape)
        shape = _spec_rounded(format_spec)
        if shape is None:
            _spec_amiss(format_spec, '')
        if shape[1] is not None and shape[5]:
            _spec_amiss(format_spec, "; can't use explicit alignment when zero-padding")
        return self._write_rounded(shape)

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


# CPython has a ratio stand under numbers.Rational outright; here it is
# claimed for that kind instead, which puts it in the numeric tower all
# the same.
numbers.Rational.register(Fraction)
