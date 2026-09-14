# Arithmetic on complex numbers, worked out in Python from the real
# functions of the math module.
#
# CPython's cmath is written in C and spells out, function by function
# and quadrant by quadrant, what every combination of infinity,
# not-a-number and signed zero answers with. None of that table is
# copied here. Each function below is the ordinary formula for the
# principal branch, so an argument whose two parts are finite gets the
# right answer to within the rounding of a few operations, while an
# argument carrying an infinity or a not-a-number gets whatever the
# formula makes of it. That is often not what the reference says, and a
# test reading those edges fails rather than being handed a convenient
# answer that was never computed.
#
# The same goes for the far ends of the finite range: a real part much
# beyond 709 makes exp overflow, so cosh, sinh and exp answer with an
# infinity where CPython raises OverflowError. tanh alone is given the
# large-argument branch, because without it the ordinary formula divides
# one infinity by another and answers not-a-number for arguments the
# reference handles.
import math

__all__ = ['acos', 'acosh', 'asin', 'asinh', 'atan', 'atanh', 'cos',
           'cosh', 'e', 'exp', 'inf', 'infj', 'isclose', 'isfinite',
           'isinf', 'isnan', 'log', 'log10', 'nan', 'nanj', 'phase',
           'pi', 'polar', 'rect', 'sin', 'sinh', 'sqrt', 'tan', 'tanh',
           'tau']

pi = math.pi
e = math.e
tau = math.tau
inf = math.inf
nan = math.nan
infj = complex(0.0, math.inf)
nanj = complex(0.0, math.nan)

# Where the real formulas stop holding: past this the exponential of a
# real overflows, so tanh is answered by its limit instead.
_large = 709.0895657128241


def _size(z):
    # The distance from zero, taken in the way that does not overflow
    # when both parts are large.
    return math.hypot(z.real, z.imag)


def phase(z):
    z = complex(z)
    return math.atan2(z.imag, z.real)


def polar(z):
    z = complex(z)
    return (_size(z), phase(z))


def rect(r, phi):
    return complex(r * math.cos(phi), r * math.sin(phi))


def exp(z):
    z = complex(z)
    size = math.exp(z.real)
    return complex(size * math.cos(z.imag), size * math.sin(z.imag))


def log(z, base=None):
    z = complex(z)
    size = _size(z)
    if size == 0.0:
        raise 'ValueError: math domain error'
    whole = complex(math.log(size), phase(z))
    if base is None:
        return whole
    return whole / log(base)


def log10(z):
    return log(z) / math.log(10.0)


def sqrt(z):
    # The square root taken through the half that keeps its accuracy:
    # one part from the size, the other from the part that is left.
    z = complex(z)
    if z.real == 0.0 and z.imag == 0.0:
        return complex(0.0, z.imag)
    side = math.sqrt((_size(z) + math.fabs(z.real)) / 2.0)
    if z.real >= 0.0:
        return complex(side, z.imag / (2.0 * side))
    return complex(math.fabs(z.imag) / (2.0 * side), math.copysign(side, z.imag))


def cos(z):
    z = complex(z)
    return complex(math.cos(z.real) * math.cosh(z.imag),
                   -math.sin(z.real) * math.sinh(z.imag))


def sin(z):
    z = complex(z)
    return complex(math.sin(z.real) * math.cosh(z.imag),
                   math.cos(z.real) * math.sinh(z.imag))


def tan(z):
    # tan turned on its side is tanh, which is where the care about
    # large arguments already lives.
    z = complex(z)
    turned = tanh(complex(-z.imag, z.real))
    return complex(turned.imag, -turned.real)


def cosh(z):
    z = complex(z)
    return complex(math.cos(z.imag) * math.cosh(z.real),
                   math.sin(z.imag) * math.sinh(z.real))


def sinh(z):
    z = complex(z)
    return complex(math.cos(z.imag) * math.sinh(z.real),
                   math.sin(z.imag) * math.cosh(z.real))


def tanh(z):
    z = complex(z)
    across = z.real
    up = z.imag
    if math.fabs(across) > _large:
        return complex(math.copysign(1.0, across),
                       4.0 * math.sin(up) * math.cos(up) * math.exp(-2.0 * math.fabs(across)))
    flat = math.tanh(across)
    tall = math.tan(up)
    narrow = 1.0 / math.cosh(across)
    both = flat * tall
    under = 1.0 + both * both
    return complex(flat * (1.0 + tall * tall) / under,
                   ((tall / under) * narrow) * narrow)


def asin(z):
    z = complex(z)
    return complex(0.0, -1.0) * log(complex(0.0, 1.0) * z + sqrt(1.0 - z * z))


def acos(z):
    z = complex(z)
    return complex(math.pi / 2.0, 0.0) - asin(z)


def atan(z):
    z = complex(z)
    half = complex(0.0, 0.5)
    return half * (log(1.0 - complex(0.0, 1.0) * z) - log(1.0 + complex(0.0, 1.0) * z))


def asinh(z):
    z = complex(z)
    return log(z + sqrt(z * z + 1.0))


def acosh(z):
    z = complex(z)
    return log(z + sqrt(z - 1.0) * sqrt(z + 1.0))


def atanh(z):
    z = complex(z)
    return (log(1.0 + z) - log(1.0 - z)) / 2.0


def isfinite(z):
    z = complex(z)
    return math.isfinite(z.real) and math.isfinite(z.imag)


def isinf(z):
    z = complex(z)
    return math.isinf(z.real) or math.isinf(z.imag)


def isnan(z):
    z = complex(z)
    return math.isnan(z.real) or math.isnan(z.imag)


def isclose(a, b, rel_tol=1e-09, abs_tol=0.0):
    a = complex(a)
    b = complex(b)
    if rel_tol < 0.0 or abs_tol < 0.0:
        raise 'ValueError: tolerances must be non-negative'
    if a == b:
        return True
    if isinf(a) or isinf(b):
        return False
    apart = _size(a - b)
    if math.isnan(apart):
        return False
    room = rel_tol * _size(b)
    other = rel_tol * _size(a)
    if other > room:
        room = other
    if abs_tol > room:
        room = abs_tol
    return apart <= room
