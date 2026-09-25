# Complex elementary functions using the scaled formulas and special-value
# tables from CPython Modules/cmathmodule.c (Kahan, C99 Annex G).
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

# Binary64 thresholds used to keep intermediate results in range.
_large_double = 4.4942328371557893e307
_sqrt_large = 6.703903964971298e153
_min_double = 2.2250738585072014e-308
_sqrt_min = 1.4916681462400413e-154
_ln2 = 0.6931471805599453


def _convert(z):
    if isinstance(z, complex):
        return z
    method = getattr(type(z), '__complex__', None)
    if method is not None:
        result = method(z)
        if not isinstance(result, complex):
            raise TypeError('__complex__ returned non-complex (type ' + type(result).__name__ + ')')
        return result
    if isinstance(z, (int, float)):
        return complex(z)
    if hasattr(type(z), '__float__'):
        return complex(float(z))
    method = getattr(type(z), '__index__', None)
    if method is not None:
        index = method(z)
        if not isinstance(index, int):
            raise TypeError('__index__ returned non-int (type ' + type(index).__name__ + ')')
        return complex(float(index))
    raise TypeError('must be real number, not ' + type(z).__name__)


def _special_type(x):
    if math.isnan(x):
        return 6
    if math.isinf(x):
        return 0 if x < 0.0 else 5
    if x == 0.0:
        return 2 if math.copysign(1.0, x) < 0.0 else 3
    return 1 if x < 0.0 else 4


def _special(z, table):
    real, imag = table[_special_type(z.real)][_special_type(z.imag)]
    return complex(real, imag)


def _nonfinite(z):
    return not math.isfinite(z.real) or not math.isfinite(z.imag)


def _range_result(real, imag):
    if math.isinf(real) or math.isinf(imag):
        raise OverflowError('math range error')
    return complex(real, imag)


def _size(z):
    # The distance from zero, taken in the way that does not overflow
    # when both parts are large.
    return math.hypot(z.real, z.imag)


def phase(z):
    z = _convert(z)
    return math.atan2(z.imag, z.real)


def polar(z):
    z = _convert(z)
    size = _size(z)
    if math.isinf(size) and not _nonfinite(z):
        raise OverflowError('math range error')
    return (size, phase(z))


def rect(r, phi):
    math._check_real(r)
    math._check_real(phi)
    r, phi = float(r), float(phi)
    if not math.isfinite(r) or not math.isfinite(phi):
        if r != 0.0 and not math.isnan(r) and math.isinf(phi):
            raise ValueError('math domain error')
        if math.isinf(r) and math.isfinite(phi) and phi != 0.0:
            real, imag = math.copysign(inf, math.cos(phi)), math.copysign(inf, math.sin(phi))
            if r < 0.0:
                real, imag = -real, -imag
            return complex(real, imag)
        return _special(complex(r, phi), _rect_special)
    if phi == 0.0:
        return complex(r, r * phi)
    return complex(r * math.cos(phi), r * math.sin(phi))


def exp(z):
    z = _convert(z)
    if _nonfinite(z):
        if math.isinf(z.imag) and (math.isfinite(z.real) or z.real == inf):
            raise ValueError('math domain error')
        if math.isinf(z.real) and math.isfinite(z.imag) and z.imag != 0.0:
            size = inf if z.real > 0.0 else 0.0
            return complex(math.copysign(size, math.cos(z.imag)), math.copysign(size, math.sin(z.imag)))
        return _special(z, _exp_special)
    if z.real > math.log(_large_double):
        size = math.exp(z.real - 1.0)
        return _range_result(size * math.cos(z.imag) * e, size * math.sin(z.imag) * e)
    size = math.exp(z.real)
    return _range_result(size * math.cos(z.imag), size * math.sin(z.imag))


def log(z, base=None):
    z = _convert(z)
    if _nonfinite(z):
        result = _special(z, _log_special)
    else:
        ax, ay = abs(z.real), abs(z.imag)
        if ax > _large_double or ay > _large_double:
            real = math.log(math.hypot(ax / 2.0, ay / 2.0)) + _ln2
        elif ax < _min_double and ay < _min_double:
            if ax == 0.0 and ay == 0.0:
                raise ValueError('math domain error')
            real = math.log(math.hypot(math.ldexp(ax, 53), math.ldexp(ay, 53))) - 53.0 * _ln2
        else:
            h = math.hypot(ax, ay)
            if 0.71 <= h and h <= 1.73:
                am, an = max(ax, ay), min(ax, ay)
                real = math.log1p((am - 1.0) * (am + 1.0) + an * an) / 2.0
            else:
                real = math.log(h)
        result = complex(real, math.atan2(z.imag, z.real))
    if base is None:
        return result
    return result / log(base)


def log10(z):
    result = log(z)
    return complex(result.real / math.log(10.0), result.imag / math.log(10.0))


def sqrt(z):
    z = _convert(z)
    if _nonfinite(z):
        return _special(z, _sqrt_special)
    if z.real == 0.0 and z.imag == 0.0:
        return complex(0.0, z.imag)
    ax, ay = abs(z.real), abs(z.imag)
    if ax < _min_double and ay < _min_double:
        ax = math.ldexp(ax, 53)
        side = math.ldexp(math.sqrt(ax + math.hypot(ax, math.ldexp(ay, 53))), -27)
    else:
        ax /= 8.0
        side = 2.0 * math.sqrt(ax + math.hypot(ax, ay / 8.0))
    other = ay / (2.0 * side)
    if z.real >= 0.0:
        return complex(side, math.copysign(other, z.imag))
    return complex(other, math.copysign(side, z.imag))


def cos(z):
    z = _convert(z)
    return cosh(complex(-z.imag, z.real))


def sin(z):
    z = _convert(z)
    result = sinh(complex(-z.imag, z.real))
    return complex(result.imag, -result.real)


def tan(z):
    # tan turned on its side is tanh, which is where the care about
    # large arguments already lives.
    z = _convert(z)
    turned = tanh(complex(-z.imag, z.real))
    return complex(turned.imag, -turned.real)


def cosh(z):
    z = _convert(z)
    if _nonfinite(z):
        if math.isinf(z.imag) and not math.isnan(z.real):
            raise ValueError('math domain error')
        if math.isinf(z.real) and math.isfinite(z.imag) and z.imag != 0.0:
            real = math.copysign(inf, math.cos(z.imag))
            imag = math.copysign(inf, math.sin(z.imag))
            if z.real < 0.0:
                imag = -imag
            return complex(real, imag)
        return _special(z, _cosh_special)
    if abs(z.real) > math.log(_large_double):
        x = z.real - math.copysign(1.0, z.real)
        return _range_result(math.cos(z.imag) * math.cosh(x) * e,
                             math.sin(z.imag) * math.sinh(x) * e)
    return _range_result(math.cos(z.imag) * math.cosh(z.real),
                         math.sin(z.imag) * math.sinh(z.real))


def sinh(z):
    z = _convert(z)
    if _nonfinite(z):
        if math.isinf(z.imag) and not math.isnan(z.real):
            raise ValueError('math domain error')
        if math.isinf(z.real) and math.isfinite(z.imag) and z.imag != 0.0:
            real = math.copysign(inf, math.cos(z.imag))
            imag = math.copysign(inf, math.sin(z.imag))
            if z.real < 0.0:
                real = -real
            return complex(real, imag)
        return _special(z, _sinh_special)
    if abs(z.real) > math.log(_large_double):
        x = z.real - math.copysign(1.0, z.real)
        return _range_result(math.cos(z.imag) * math.sinh(x) * e,
                             math.sin(z.imag) * math.cosh(x) * e)
    return _range_result(math.cos(z.imag) * math.sinh(z.real),
                         math.sin(z.imag) * math.cosh(z.real))


def tanh(z):
    z = _convert(z)
    if _nonfinite(z):
        if math.isinf(z.imag) and math.isfinite(z.real):
            raise ValueError('math domain error')
        if math.isinf(z.real) and math.isfinite(z.imag) and z.imag != 0.0:
            return complex(math.copysign(1.0, z.real), math.copysign(0.0, 2.0 * math.sin(z.imag) * math.cos(z.imag)))
        return _special(z, _tanh_special)
    across = z.real
    up = z.imag
    if math.fabs(across) > math.log(_large_double):
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
    z = _convert(z)
    result = asinh(complex(-z.imag, z.real))
    return complex(result.imag, -result.real)


def acos(z):
    z = _convert(z)
    if _nonfinite(z):
        return _special(z, _acos_special)
    if abs(z.real) > _large_double or abs(z.imag) > _large_double:
        return complex(math.atan2(abs(z.imag), z.real),
                       -math.copysign(math.log(math.hypot(z.real / 2.0, z.imag / 2.0)) + 2.0 * _ln2, z.imag))
    s1 = sqrt(complex(1.0 - z.real, -z.imag))
    s2 = sqrt(complex(1.0 + z.real, z.imag))
    return complex(2.0 * math.atan2(s1.real, s2.real),
                   math.asinh(s2.real * s1.imag - s2.imag * s1.real))


def atan(z):
    z = _convert(z)
    result = atanh(complex(-z.imag, z.real))
    return complex(result.imag, -result.real)


def asinh(z):
    z = _convert(z)
    if _nonfinite(z):
        return _special(z, _asinh_special)
    if abs(z.real) > _large_double or abs(z.imag) > _large_double:
        size = math.log(math.hypot(z.real / 2.0, z.imag / 2.0)) + 2.0 * _ln2
        real = math.copysign(size, z.real) if z.imag >= 0.0 else -math.copysign(size, -z.real)
        return complex(real, math.atan2(z.imag, abs(z.real)))
    s1 = sqrt(complex(1.0 + z.imag, -z.real))
    s2 = sqrt(complex(1.0 - z.imag, z.real))
    return complex(math.asinh(s1.real * s2.imag - s2.real * s1.imag),
                   math.atan2(z.imag, s1.real * s2.real - s1.imag * s2.imag))


def acosh(z):
    z = _convert(z)
    if _nonfinite(z):
        return _special(z, _acosh_special)
    if abs(z.real) > _large_double or abs(z.imag) > _large_double:
        return complex(math.log(math.hypot(z.real / 2.0, z.imag / 2.0)) + 2.0 * _ln2,
                       math.atan2(z.imag, z.real))
    s1 = sqrt(complex(z.real - 1.0, z.imag))
    s2 = sqrt(complex(z.real + 1.0, z.imag))
    return complex(math.asinh(s1.real * s2.real + s1.imag * s2.imag),
                   2.0 * math.atan2(s1.imag, s2.real))


def atanh(z):
    z = _convert(z)
    if _nonfinite(z):
        return _special(z, _atanh_special)
    if z.real < 0.0:
        result = atanh(complex(-z.real, -z.imag))
        return complex(-result.real, -result.imag)
    ay = abs(z.imag)
    if z.real > _sqrt_large or ay > _sqrt_large:
        h = math.hypot(z.real / 2.0, z.imag / 2.0)
        return complex(z.real / 4.0 / h / h, math.copysign(pi / 2.0, z.imag))
    if z.real == 1.0 and ay < _sqrt_min:
        if ay == 0.0:
            raise ValueError('math domain error')
        return complex(-math.log(math.sqrt(ay) / math.sqrt(math.hypot(ay, 2.0))),
                       math.copysign(math.atan2(2.0, -ay) / 2.0, z.imag))
    return complex(math.log1p(4.0 * z.real / ((1.0 - z.real) * (1.0 - z.real) + ay * ay)) / 4.0,
                   -math.atan2(-2.0 * z.imag, (1.0 - z.real) * (1.0 + z.real) - ay * ay) / 2.0)


def isfinite(z):
    z = _convert(z)
    return math.isfinite(z.real) and math.isfinite(z.imag)


def isinf(z):
    z = _convert(z)
    return math.isinf(z.real) or math.isinf(z.imag)


def isnan(z):
    z = _convert(z)
    return math.isnan(z.real) or math.isnan(z.imag)


class _IsClose:
    # Callable instances, like native module functions, do not bind a
    # receiver when stored in a class dictionary.
    def __call__(self, a, b, *, rel_tol=1e-09, abs_tol=0.0):
        a, b = _convert(a), _convert(b)
        math._check_real(rel_tol)
        math._check_real(abs_tol)
        rel_tol, abs_tol = float(rel_tol), float(abs_tol)
        if rel_tol < 0.0 or abs_tol < 0.0:
            raise ValueError('tolerances must be non-negative')
        if a == b:
            return True
        if isinf(a) or isinf(b):
            return False
        apart = _size(a - b)
        return apart <= rel_tol * _size(b) or apart <= rel_tol * _size(a) or apart <= abs_tol


isclose = _IsClose()


# Entries marked None are finite cases, handled by the formulas above.
_acos_special = [
  [ [(0.75*pi),inf], [pi,inf],  [pi,inf],  [pi,-inf],  [pi,-inf],  [(0.75*pi),-inf], [nan,inf] ],
  [ [(0.5*pi),inf], [None,None],    [None,None],    [None,None],     [None,None],     [(0.5*pi),-inf], [nan,nan] ],
  [ [(0.5*pi),inf], [None,None],    [(0.5*pi),0.], [(0.5*pi),-0.], [None,None],     [(0.5*pi),-inf], [(0.5*pi),nan] ],
  [ [(0.5*pi),inf], [None,None],    [(0.5*pi),0.], [(0.5*pi),-0.], [None,None],     [(0.5*pi),-inf], [(0.5*pi),nan] ],
  [ [(0.5*pi),inf], [None,None],    [None,None],    [None,None],     [None,None],     [(0.5*pi),-inf], [nan,nan] ],
  [ [(0.25*pi),inf], [0.,inf], [0.,inf], [0.,-inf], [0.,-inf], [(0.25*pi),-inf], [nan,inf] ],
  [ [nan,inf],   [nan,nan],    [nan,nan],    [nan,nan],     [nan,nan],     [nan,-inf],   [nan,nan] ]
]

_acosh_special = [
  [ [inf,-(0.75*pi)], [inf,-pi],  [inf,-pi],  [inf,pi],  [inf,pi],  [inf,(0.75*pi)], [inf,nan] ],
  [ [inf,-(0.5*pi)], [None,None],     [None,None],     [None,None],    [None,None],    [inf,(0.5*pi)], [nan,nan] ],
  [ [inf,-(0.5*pi)], [None,None],     [0.,-(0.5*pi)], [0.,(0.5*pi)], [None,None],    [inf,(0.5*pi)], [nan,(0.5*pi)] ],
  [ [inf,-(0.5*pi)], [None,None],     [0.,-(0.5*pi)], [0.,(0.5*pi)], [None,None],    [inf,(0.5*pi)], [nan,(0.5*pi)] ],
  [ [inf,-(0.5*pi)], [None,None],     [None,None],     [None,None],    [None,None],    [inf,(0.5*pi)], [nan,nan] ],
  [ [inf,-(0.25*pi)], [inf,-0.], [inf,-0.], [inf,0.], [inf,0.], [inf,(0.25*pi)], [inf,nan] ],
  [ [inf,nan],    [nan,nan],     [nan,nan],     [nan,nan],    [nan,nan],    [inf,nan],   [nan,nan] ]
]

_asinh_special = [
  [ [-inf,-(0.25*pi)], [-inf,-0.], [-inf,-0.], [-inf,0.], [-inf,0.], [-inf,(0.25*pi)], [-inf,nan] ],
  [ [-inf,-(0.5*pi)], [None,None],      [None,None],      [None,None],     [None,None],     [-inf,(0.5*pi)], [nan,nan] ],
  [ [-inf,-(0.5*pi)], [None,None],      [-0.,-0.],  [-0.,0.],  [None,None],     [-inf,(0.5*pi)], [nan,nan] ],
  [ [inf,-(0.5*pi)],  [None,None],      [0.,-0.],   [0.,0.],   [None,None],     [inf,(0.5*pi)],  [nan,nan] ],
  [ [inf,-(0.5*pi)],  [None,None],      [None,None],      [None,None],     [None,None],     [inf,(0.5*pi)],  [nan,nan] ],
  [ [inf,-(0.25*pi)],  [inf,-0.],  [inf,-0.],  [inf,0.],  [inf,0.],  [inf,(0.25*pi)],  [inf,nan] ],
  [ [inf,nan],     [nan,nan],      [nan,-0.],    [nan,0.],    [nan,nan],     [inf,nan],    [nan,nan] ]
]

_atanh_special = [
  [ [-0.,-(0.5*pi)], [-0.,-(0.5*pi)], [-0.,-(0.5*pi)], [-0.,(0.5*pi)], [-0.,(0.5*pi)], [-0.,(0.5*pi)], [-0.,nan] ],
  [ [-0.,-(0.5*pi)], [None,None],      [None,None],      [None,None],     [None,None],     [-0.,(0.5*pi)], [nan,nan] ],
  [ [-0.,-(0.5*pi)], [None,None],      [-0.,-0.],  [-0.,0.],  [None,None],     [-0.,(0.5*pi)], [-0.,nan] ],
  [ [0.,-(0.5*pi)],  [None,None],      [0.,-0.],   [0.,0.],   [None,None],     [0.,(0.5*pi)],  [0.,nan] ],
  [ [0.,-(0.5*pi)],  [None,None],      [None,None],      [None,None],     [None,None],     [0.,(0.5*pi)],  [nan,nan] ],
  [ [0.,-(0.5*pi)],  [0.,-(0.5*pi)],  [0.,-(0.5*pi)],  [0.,(0.5*pi)],  [0.,(0.5*pi)],  [0.,(0.5*pi)],  [0.,nan] ],
  [ [0.,-(0.5*pi)],  [nan,nan],      [nan,nan],      [nan,nan],     [nan,nan],     [0.,(0.5*pi)],  [nan,nan] ]
]

_cosh_special = [
  [ [inf,nan], [None,None], [inf,0.],  [inf,-0.], [None,None], [inf,nan], [inf,nan] ],
  [ [nan,nan],   [None,None], [None,None],     [None,None],     [None,None], [nan,nan],   [nan,nan] ],
  [ [nan,0.],  [None,None], [1.,0.],   [1.,-0.],  [None,None], [nan,0.],  [nan,0.] ],
  [ [nan,0.],  [None,None], [1.,-0.],  [1.,0.],   [None,None], [nan,0.],  [nan,0.] ],
  [ [nan,nan],   [None,None], [None,None],     [None,None],     [None,None], [nan,nan],   [nan,nan] ],
  [ [inf,nan], [None,None], [inf,-0.], [inf,0.],  [None,None], [inf,nan], [inf,nan] ],
  [ [nan,nan],   [nan,nan], [nan,0.],    [nan,0.],    [nan,nan], [nan,nan],   [nan,nan] ]
]

_exp_special = [
  [ [0.,0.], [None,None], [0.,-0.],  [0.,0.],  [None,None], [0.,0.], [0.,0.] ],
  [ [nan,nan],   [None,None], [None,None],     [None,None],    [None,None], [nan,nan],   [nan,nan] ],
  [ [nan,nan],   [None,None], [1.,-0.],  [1.,0.],  [None,None], [nan,nan],   [nan,nan] ],
  [ [nan,nan],   [None,None], [1.,-0.],  [1.,0.],  [None,None], [nan,nan],   [nan,nan] ],
  [ [nan,nan],   [None,None], [None,None],     [None,None],    [None,None], [nan,nan],   [nan,nan] ],
  [ [inf,nan], [None,None], [inf,-0.], [inf,0.], [None,None], [inf,nan], [inf,nan] ],
  [ [nan,nan],   [nan,nan], [nan,-0.],   [nan,0.],   [nan,nan], [nan,nan],   [nan,nan] ]
]

_log_special = [
  [ [inf,-(0.75*pi)], [inf,-pi],  [inf,-pi],   [inf,pi],   [inf,pi],  [inf,(0.75*pi)],  [inf,nan] ],
  [ [inf,-(0.5*pi)], [None,None],     [None,None],      [None,None],     [None,None],    [inf,(0.5*pi)],  [nan,nan] ],
  [ [inf,-(0.5*pi)], [None,None],     [-inf,-pi],  [-inf,pi],  [None,None],    [inf,(0.5*pi)],  [nan,nan] ],
  [ [inf,-(0.5*pi)], [None,None],     [-inf,-0.], [-inf,0.], [None,None],    [inf,(0.5*pi)],  [nan,nan] ],
  [ [inf,-(0.5*pi)], [None,None],     [None,None],      [None,None],     [None,None],    [inf,(0.5*pi)],  [nan,nan] ],
  [ [inf,-(0.25*pi)], [inf,-0.], [inf,-0.],  [inf,0.],  [inf,0.], [inf,(0.25*pi)],  [inf,nan] ],
  [ [inf,nan],    [nan,nan],     [nan,nan],      [nan,nan],     [nan,nan],    [inf,nan],    [nan,nan] ]
]

_sinh_special = [
  [ [inf,nan], [None,None], [-inf,-0.], [-inf,0.], [None,None], [inf,nan], [inf,nan] ],
  [ [nan,nan],   [None,None], [None,None],      [None,None],     [None,None], [nan,nan],   [nan,nan] ],
  [ [0.,nan],  [None,None], [-0.,-0.],  [-0.,0.],  [None,None], [0.,nan],  [0.,nan] ],
  [ [0.,nan],  [None,None], [0.,-0.],   [0.,0.],   [None,None], [0.,nan],  [0.,nan] ],
  [ [nan,nan],   [None,None], [None,None],      [None,None],     [None,None], [nan,nan],   [nan,nan] ],
  [ [inf,nan], [None,None], [inf,-0.],  [inf,0.],  [None,None], [inf,nan], [inf,nan] ],
  [ [nan,nan],   [nan,nan], [nan,-0.],    [nan,0.],    [nan,nan], [nan,nan],   [nan,nan] ]
]

_sqrt_special = [
  [ [inf,-inf], [0.,-inf], [0.,-inf], [0.,inf], [0.,inf], [inf,inf], [nan,inf] ],
  [ [inf,-inf], [None,None],     [None,None],     [None,None],    [None,None],    [inf,inf], [nan,nan] ],
  [ [inf,-inf], [None,None],     [0.,-0.],  [0.,0.],  [None,None],    [inf,inf], [nan,nan] ],
  [ [inf,-inf], [None,None],     [0.,-0.],  [0.,0.],  [None,None],    [inf,inf], [nan,nan] ],
  [ [inf,-inf], [None,None],     [None,None],     [None,None],    [None,None],    [inf,inf], [nan,nan] ],
  [ [inf,-inf], [inf,-0.], [inf,-0.], [inf,0.], [inf,0.], [inf,inf], [inf,nan] ],
  [ [inf,-inf], [nan,nan],     [nan,nan],     [nan,nan],    [nan,nan],    [inf,inf], [nan,nan] ]
]

_tanh_special = [
  [ [-1.,0.], [None,None], [-1.,-0.], [-1.,0.], [None,None], [-1.,0.], [-1.,0.] ],
  [ [nan,nan],    [None,None], [None,None],     [None,None],    [None,None], [nan,nan],    [nan,nan] ],
  [ [-0.0,nan], [None,None], [-0.,-0.], [-0.,0.], [None,None], [-0.0,nan], [-0.,nan] ],
  [ [0.0,nan],  [None,None], [0.,-0.],  [0.,0.],  [None,None], [0.0,nan],  [0.,nan] ],
  [ [nan,nan],    [None,None], [None,None],     [None,None],    [None,None], [nan,nan],    [nan,nan] ],
  [ [1.,0.],  [None,None], [1.,-0.],  [1.,0.],  [None,None], [1.,0.],  [1.,0.] ],
  [ [nan,nan],    [nan,nan], [nan,-0.],   [nan,0.],   [nan,nan], [nan,nan],    [nan,nan] ]
]

_rect_special = [
  [ [inf,nan], [None,None], [-inf,0.], [-inf,-0.], [None,None], [inf,nan], [inf,nan] ],
  [ [nan,nan],   [None,None], [None,None],     [None,None],      [None,None], [nan,nan],   [nan,nan] ],
  [ [0.,0.], [None,None], [-0.,0.],  [-0.,-0.],  [None,None], [0.,0.], [0.,0.] ],
  [ [0.,0.], [None,None], [0.,-0.],  [0.,0.],    [None,None], [0.,0.], [0.,0.] ],
  [ [nan,nan],   [None,None], [None,None],     [None,None],      [None,None], [nan,nan],   [nan,nan] ],
  [ [inf,nan], [None,None], [inf,-0.], [inf,0.],   [None,None], [inf,nan], [inf,nan] ],
  [ [nan,nan],   [nan,nan], [nan,0.],    [nan,0.],     [nan,nan], [nan,nan],   [nan,nan] ]
]
