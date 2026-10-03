# Reals use the kernel's width; integer work stays exact.
pi = 3.141592653589793
e = 2.718281828459045
inf = __math('fdiv', 1.0, 0.0)
nan = float('nan')

def _overflow_guard(x):
    # A whole number too great for any real of the width to hold is
    # carried there before its domain is ever asked about, the way
    # every other real-valued working carries it, and answers the
    # same fault such a carrying answers rather than one of its own.
    if type(x) == type(1) or type(x) == type(True):
        float(x)

def sqrt(x):
    _overflow_guard(x)
    if x < 0:
        raise ValueError('expected a nonnegative input, got ' + str(float(x)))
    return __math('sqrt', x)

def fabs(x):
    _check_real(x)
    x = float(x)
    if x == 0:
        return __math('fdiv', 0.0, 1.0)
    if x < 0:
        return __math('fdiv', -x, 1.0)
    return __math('fdiv', x, 1.0)

def _check_real(x):
    if type(x) == type(1) or type(x) == type(1.0) or type(x) == type(True):
        return
    if hasattr(x, '__float__') or hasattr(x, '__index__'):
        return
    raise 'TypeError: must be real number, not ' + type(x).__name__

def _answers_own(x, name):
    if type(x) == type(1) or type(x) == type(1.0) or type(x) == type(True):
        return False
    return hasattr(type(x), name)

def floor(x):
    if _answers_own(x, '__floor__'):
        method = x.__floor__
        if method is None:
            raise TypeError("'NoneType' object is not callable")
        answer = method()
        if type(answer) != type(1) and type(answer) != type(True):
            raise 'TypeError: __floor__ returned non-Integral (type ' + type(answer).__name__ + ')'
        return answer
    _check_real(x)
    x = float(x)
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer floor'
    n = int(x)
    if n > x:
        n -= 1
    return n

def ceil(x):
    if _answers_own(x, '__ceil__'):
        method = x.__ceil__
        if method is None:
            raise TypeError("'NoneType' object is not callable")
        answer = method()
        if type(answer) != type(1) and type(answer) != type(True):
            raise 'TypeError: __ceil__ returned non-Integral (type ' + type(answer).__name__ + ')'
        return answer
    _check_real(x)
    x = float(x)
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer ceiling'
    n = int(x)
    if n < x:
        n += 1
    return n

def trunc(x):
    if not isinstance(x, (int, float)) and not _answers_own(x, '__trunc__'):
        raise TypeError("type '" + type(x).__name__ + "' doesn't define __trunc__ method")
    if _answers_own(x, '__trunc__'):
        method = x.__trunc__
        if method is None:
            raise TypeError("'NoneType' object is not callable")
        answer = method()
        if type(answer) != type(1) and type(answer) != type(True):
            raise 'TypeError: __trunc__ returned non-Integral (type ' + type(answer).__name__ + ')'
        return answer
    if type(x) != type(1) and type(x) != type(1.0) and type(x) != type(True):
        raise "TypeError: type " + type(x).__name__ + " doesn't define __trunc__ method"
    _check_real(x)
    x = float(x)
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer truncation'
    return int(x)

def pow(x, y):
    _check_real(x)
    _check_real(y)
    # Zero to a finite power below nought is a division by nought, and
    # a base below nought to a power with a fraction to it has no real
    # answer; the width would otherwise answer nan or fault on its own
    # account without ever saying which of these it means.
    if x == 0 and isfinite(y) and y < 0:
        raise 'ValueError: math domain error'
    if x < 0 and isfinite(x) and isfinite(y) and y != int(y):
        raise 'ValueError: math domain error'
    return __math('pow', x, y)

def exp(x):
    _check_real(x)
    return __math('exp', x)

def _is_integral(x):
    return type(x) == type(1) or type(x) == type(True)

def _index_or_none(x):
    # A whole number itself, or whatever stands in an index's place;
    # anything else answers with nothing rather than a fault, since
    # what comes of it decides on its own what the value must be.
    if _is_integral(x):
        return int(x)
    if hasattr(x, '__index__'):
        return x.__index__()
    return None

def _as_index(value):
    at = _index_or_none(value)
    if at is None:
        raise 'TypeError: ' + repr(type(value).__name__) + " object cannot be interpreted as an integer"
    return at

def _int_frexp(n):
    # A whole number as a real in [1, 2) times two raised to a count,
    # the way a binary width itself is read apart, but carried out
    # exactly whatever the count of digits. The division is asked of
    # the language's own exact ratio, never of a working handed the
    # whole number outright, since that would carry it to the width by
    # itself first and overflow there before the ratio was ever taken.
    e = n.bit_length()
    return n / (1 << e), e

def _log_int(xi, working, per_bit):
    if xi <= 0:
        raise ValueError('expected a positive input')
    m, e = _int_frexp(xi)
    return __math(working, m) + e * per_bit

def _log_value(x, working, per_bit):
    # A whole number's own working is asked of its bits directly, and
    # asked first, since converting one to a real first is what would
    # overflow the width where a real never does. Anything else is
    # asked for its real; only where the real overflows the width is
    # its index asked for instead, the way a real itself is asked for
    # only where the whole number already answers to more than one.
    if _is_integral(x):
        return _log_int(int(x), working, per_bit)
    if hasattr(x, '__float__'):
        try:
            real = float(x)
        except OverflowError:
            xi = _index_or_none(x)
            if xi is None:
                raise
            return _log_int(xi, working, per_bit)
        if real <= 0:
            raise ValueError('expected a positive input, got ' + str(real))
        return __math(working, real)
    xi = _index_or_none(x)
    if xi is not None:
        return _log_int(xi, working, per_bit)
    if x <= 0:
        raise ValueError('expected a positive input, got ' + str(float(x)))
    return __math(working, x)

def log(x, base=None):
    answer = _log_value(x, 'log', 0.6931471805599453)
    if base is not None:
        if base == 1:
            raise 'ValueError: invalid logarithm base'
        answer = __math('fdiv', answer, _log_value(base, 'log', 0.6931471805599453))
    return answer

def isnan(x):
    return x != x

def isinf(x):
    return x == inf or x == -inf

def isfinite(x):
    return not isnan(x) and not isinf(x)

def isclose(a, b, rel_tol=0.000000001, abs_tol=0.0):
    _check_real(a)
    _check_real(b)
    _check_real(rel_tol)
    _check_real(abs_tol)
    a, b = float(a), float(b)
    rel_tol, abs_tol = float(rel_tol), float(abs_tol)
    if rel_tol < 0 or abs_tol < 0:
        raise 'ValueError: tolerances must be non-negative'
    if a == b:
        return True
    if isinf(a) or isinf(b):
        return False
    difference = fabs(a - b)
    scale = fabs(a)
    other = fabs(b)
    if other > scale:
        scale = other
    return difference <= abs_tol or difference <= rel_tol * scale

def copysign(x, y):
    _check_real(x)
    _check_real(y)
    return __math('copysign', float(x), float(y))

def gcd(*integers):
    result = 0
    for raw in integers:
        value = _as_index(raw)
        if value < 0:
            value = -value
        while value != 0:
            result, value = value, result % value
    return result

def lcm(*integers):
    # The least common multiple, built up one value at a time from the
    # greatest common divisor; a zero anywhere makes the whole thing
    # zero, and asking with nothing gives one, as CPython has it.
    result = 1
    for raw in integers:
        value = _as_index(raw)
        if value < 0:
            value = -value
        if value == 0 or result == 0:
            result = 0
        else:
            result = result * value // gcd(result, value)
    return result

def factorial(n):
    from operator import index
    n = index(n)
    if n < 0:
        raise 'ValueError: factorial() not defined for negative values'
    value = 1
    for i in range(2, n + 1):
        value *= i
    return value

def fsum(values):
    # Compensation keeps small terms which a plain sum would lose. A
    # value past every number is carried apart from this, both because
    # working it into the compensation would answer nan of its own
    # account (inf met by however the terms around it happen to fall)
    # and because two of opposite sign here have no sum to agree on.
    partials = []
    pos_inf = False
    neg_inf = False
    saw_nan = False
    for x in values:
        ready = __math('fsum_finite', x, partials)
        if ready is not None:
            partials, x = ready
            continue
        if type(x) != type(1.0):
            _check_real(x)
            x = float(x)
        if x != x:
            saw_nan = True
            continue
        if x == inf:
            pos_inf = True
            continue
        if x == -inf:
            neg_inf = True
            continue
        step = __math('fsum_partial', x, partials)
        if step is None:
            kept = []
            for y in partials:
                if (-x if x < 0 else x) < (-y if y < 0 else y):
                    x, y = y, x
                high = x + y
                low = y - (high - x)
                if low != 0:
                    kept.append(low)
                x = high
            kept.append(x)
        else:
            kept, x = step
        partials = kept
    # Two infinities of opposite sign have no sum to agree on, and that
    # objection stands even when a NaN walked in among them; CPython says
    # so before it answers a NaN of its own.
    if pos_inf and neg_inf:
        raise 'ValueError: -inf + inf in fsum'
    if saw_nan:
        return nan
    if pos_inf:
        return inf
    if neg_inf:
        return -inf
    # The partials are carried smallest first; folded back together
    # from the top, the way they were built, a tie is carried past the
    # figure it lands on when what is left still leans the same way.
    count = len(partials)
    high = 0.0
    low = 0.0
    if count > 0:
        count -= 1
        high = partials[count]
        while count > 0:
            x = high
            count -= 1
            y = partials[count]
            high = x + y
            residue = high - x
            low = y - residue
            if low != 0:
                break
        if count > 0 and ((low < 0 and partials[count - 1] < 0) or (low > 0 and partials[count - 1] > 0)):
            doubled = low * 2.0
            nudged = high + doubled
            if doubled == nudged - high:
                high = nudged
    if not isfinite(high):
        # Every value handed in was finite, so a working that came out
        # otherwise did so only by outgrowing the width along the way.
        raise 'OverflowError: intermediate overflow in fsum'
    return __math('fdiv', high, 1.0)

isclose = staticmethod(isclose)

def prod(values, *, start=1):
    for x in values:
        start *= x
    return start

tau = 2 * pi

def comb(n, k):
    from operator import index
    n, k = index(n), index(k)
    if n < 0 or k < 0:
        raise 'ValueError: comb arguments must be non-negative'
    if k > n:
        return 0
    k = k if k < n - k else n - k
    result = 1
    for i in range(1, k + 1):
        result = result * (n - k + i) // i
    return result

def perm(n, k=None):
    if k is None:
        return factorial(n)
    from operator import index
    n, k = index(n), index(k)
    if n < 0 or k < 0:
        raise 'ValueError: perm arguments must be non-negative'
    if k > n:
        return 0
    result = 1
    for i in range(n - k + 1, n + 1):
        result *= i
    return result

def isqrt(n):
    from operator import index
    n = index(n)
    if n < 0:
        raise 'ValueError: isqrt argument must be non-negative'
    low = 0
    high = n + 1
    while high - low > 1:
        mid = (low + high) // 2
        if mid * mid <= n:
            low = mid
        else:
            high = mid
    return low

def hypot(*coordinates):
    values = []
    for coordinate in coordinates:
        if type(coordinate) != type(1.0):
            _check_real(coordinate)
        values.append(float(coordinate))
    return _hypot_values(values)


def _hypot_values(values):
    # Both callers have already converted every coordinate. Keep that
    # ordering even when an early coordinate is infinite or NaN: a later
    # conversion can still raise. Inspecting these base floats directly
    # avoids repeating the public conversion protocol for each distance.
    saw_inf = False
    saw_nan = False
    for value in values:
        saw_inf = saw_inf or value == inf or value == -inf
        saw_nan = saw_nan or value != value
    if saw_inf:
        return inf
    if saw_nan:
        return nan
    if len(values) == 0:
        return 0.0
    if len(values) == 1:
        return fabs(values[0])
    import decimal
    squares = decimal.Decimal(0)
    for value in values:
        exact = decimal.Decimal(value)
        squares += exact * exact
    if squares == 0:
        return 0.0
    try:
        return float(squares.sqrt())
    except OverflowError:
        return inf


def _point_items(point):
    # An exact tuple already is an immutable snapshot. Subclasses still
    # take the iterator path so that their iteration overrides are heard.
    kind = type(point)
    if kind == type(()):
        return point
    if kind == type([]):
        return list(point)
    items = []
    if type(point).__name__ in ('generator', 'list_iterator', 'tuple_iterator'):
        iterator = point
    else:
        iterator = iter(point)
    while True:
        try:
            value = next(iterator)
        except StopIteration:
            return items
        items.append(value)

def dist(p, q, /):
    p, q = _point_items(p), _point_items(q)
    if len(p) != len(q):
        raise 'ValueError: both points must have the same number of dimensions'
    ready = __math('dist_float', p, q)
    if ready is not None:
        kind, differences = ready
        if kind == 1:
            return inf
        if kind == 2:
            return nan
        return _hypot_values(differences)
    differences = []
    for i in range(len(p)):
        if type(p[i]) != type(1.0):
            _check_real(p[i])
        if type(q[i]) != type(1.0):
            _check_real(q[i])
        differences.append(float(p[i]) - float(q[i]))
    return _hypot_values(differences)

def log2(x):
    return _log_value(x, 'log2', 1.0)

def log10(x):
    return _log_value(x, 'log10', 0.3010299956639812)

def log1p(x):
    _overflow_guard(x)
    if x <= -1:
        raise 'ValueError: math domain error'
    return __math('log1p', x)

def expm1(x):
    return __math('expm1', x)

def degrees(x):
    return __math('fdiv', x * 180, pi)

def radians(x):
    return __math('fdiv', x * pi, 180)

def sin(x):
    if isinf(x):
        raise 'ValueError: math domain error'
    return __math('sin', x)

def cos(x):
    if isinf(x):
        raise 'ValueError: math domain error'
    return __math('cos', x)

def tan(x):
    if isinf(x):
        raise 'ValueError: math domain error'
    return __math('tan', x)

def asin(x):
    _overflow_guard(x)
    if x < -1 or x > 1:
        raise 'ValueError: math domain error'
    return __math('asin', x)

def acos(x):
    _overflow_guard(x)
    if x < -1 or x > 1:
        raise 'ValueError: math domain error'
    return __math('acos', x)

def atan(x):
    return __math('atan', x)

def atan2(y, x):
    _check_real(y)
    _check_real(x)
    return __math('atan2', y, x)

def sinh(x):
    return __math('sinh', x)

def cosh(x):
    return __math('cosh', x)

def tanh(x):
    return __math('tanh', x)

def asinh(x):
    return __math('asinh', x)

def acosh(x):
    _overflow_guard(x)
    if x < 1:
        raise 'ValueError: math domain error'
    return __math('acosh', x)

def atanh(x):
    _overflow_guard(x)
    if x <= -1 or x >= 1:
        raise ValueError('expected a number between -1 and 1, got ' + str(float(x)))
    # The width's own atanh is not trusted to the last place on the
    # negative side, so the working is done by hand the way the careful
    # libraries do it: half of log1p of twice the ratio, the sign handed
    # back at the end so that a signed zero keeps its sign.
    ax = -x if x < 0 else x
    if ax < 0.5:
        t = ax + ax
        t = 0.5 * __math('log1p', t + t * ax / (1.0 - ax))
    else:
        t = 0.5 * __math('log1p', (ax + ax) / (1.0 - ax))
    return __math('copysign', t, x)

# erf and erfc follow the paths CPython's own fallback takes: a power
# series close to zero and a continued fraction further out, joined at
# |x| = 1.5. gamma and lgamma use the Lanczos approximation with the
# coefficients CPython carries, a reflection through sin(pi*x) answering
# for the negative half of the line. The special values and faults are
# CPython's throughout: a NaN comes back untouched, an infinite or
# whole-number input is answered on its own terms, and everything else
# is left to the approximation.
_lanczos_g = 6.02468004077673
_lanczos_g_minus_half = 5.52468004077673
_lanczos_num = [23531376880.41076, 42919803642.6491, 35711959237.35567,
                17921034426.03721, 6039542586.352028, 1439720407.3117216,
                248874557.86205417, 31426415.585400194, 2876370.6289353725,
                186056.26539522348, 8071.672002365816, 210.82427775157936,
                2.5066282746310002]
_lanczos_den = [0.0, 39916800.0, 120543840.0, 150917976.0, 105258076.0,
                45995730.0, 13339535.0, 2637558.0, 357423.0, 32670.0,
                1925.0, 66.0, 1.0]
_gamma_integral = [1.0, 1.0, 2.0, 6.0, 24.0, 120.0, 720.0, 5040.0, 40320.0,
                   362880.0, 3628800.0, 39916800.0, 479001600.0, 6227020800.0,
                   87178291200.0, 1307674368000.0, 20922789888000.0,
                   355687428096000.0, 6402373705728000.0, 121645100408832000.0,
                   2432902008176640000.0, 51090942171709440000.0,
                   1124000727777607680000.0]
_logpi = 1.1447298858494002
_sqrtpi = 1.772453850905516

def _sinpi(x):
    # sin(pi*x) for a finite x. Whole and half turns are folded away
    # before the sine is ever asked, so an exact multiple answers an
    # exact zero; the sign the argument brought is handed back at the end.
    y = __math('fmod', __math('copysign', x, 1.0), 2.0)
    n = int(2.0 * y + 0.5)
    if n == 0:
        r = __math('sin', pi * y)
    elif n == 1:
        r = __math('cos', pi * (y - 0.5))
    elif n == 2:
        r = __math('sin', pi * (1.0 - y))
    elif n == 3:
        r = -__math('cos', pi * (y - 1.5))
    else:
        r = __math('sin', pi * (y - 2.0))
    return __math('copysign', 1.0, x) * r

def _lanczos_sum(x):
    # The Lanczos sum as a ratio of two polynomials, counted out from
    # the far end for small x and as a ratio in 1/x past it, where the
    # straight reading would drown the small coefficients.
    num = 0.0
    den = 0.0
    if x < 5.0:
        i = 12
        while i >= 0:
            num = num * x + _lanczos_num[i]
            den = den * x + _lanczos_den[i]
            i -= 1
    else:
        for i in range(13):
            num = num / x + _lanczos_num[i]
            den = den / x + _lanczos_den[i]
    return num / den

def gamma(x):
    _check_real(x)
    _overflow_guard(x)
    x = float(x)
    if isnan(x):
        return x
    if isinf(x):
        if x > 0:
            return x
        raise ValueError('expected a noninteger or positive integer, got ' + str(x))
    if x == int(x):
        if x <= 0:
            raise ValueError('expected a noninteger or positive integer, got ' + str(x))
        if x <= 23:
            return _gamma_integral[int(x) - 1]
    absx = -x if x < 0 else x
    # Near zero the answer is 1/x to the width's accuracy; past two
    # hundred it has either overflown or dwindled to a signed zero.
    if absx < 1e-20:
        r = __math('fdiv', 1.0, x)
        if isinf(r):
            raise 'OverflowError: math range error'
        return r
    if absx > 200.0:
        if x < 0:
            return __math('fdiv', 0.0, _sinpi(x))
        raise 'OverflowError: math range error'
    y = absx + _lanczos_g_minus_half
    # The error in the computed x + g - 1/2, folded back into the answer
    # below: where the width cannot hold the sum exactly, pow and exp
    # would otherwise grow that small error into many last places.
    if absx > _lanczos_g_minus_half:
        q = y - absx
        z = q - _lanczos_g_minus_half
    else:
        q = y - _lanczos_g_minus_half
        z = q - absx
    z = z * _lanczos_g / y
    if x < 0:
        r = -pi / _sinpi(absx) / absx * __math('exp', y) / _lanczos_sum(absx)
        r = r - z * r
        if absx < 140.0:
            r = r / __math('pow', y, absx - 0.5)
        else:
            halved = __math('pow', y, absx / 2.0 - 0.25)
            r = r / halved
            r = r / halved
    else:
        r = _lanczos_sum(absx) / __math('exp', y)
        r = r + z * r
        if absx < 140.0:
            r = r * __math('pow', y, absx - 0.5)
        else:
            halved = __math('pow', y, absx / 2.0 - 0.25)
            r = r * halved
            r = r * halved
    if isinf(r):
        raise 'OverflowError: math range error'
    return __math('fdiv', r, 1.0)

def lgamma(x):
    _check_real(x)
    _overflow_guard(x)
    x = float(x)
    if isnan(x):
        return x
    if isinf(x):
        return inf
    if x <= 2.0 and x == int(x):
        if x <= 0:
            raise ValueError('expected a noninteger or positive integer, got ' + str(x))
        return __math('fdiv', 0.0, 1.0)
    absx = -x if x < 0 else x
    if absx < 1e-20:
        return -__math('log', absx)
    r = __math('log', _lanczos_sum(absx)) - _lanczos_g
    r = r + (absx - 0.5) * (__math('log', absx + _lanczos_g - 0.5) - 1)
    if x < 0:
        s = _sinpi(absx)
        r = _logpi - __math('log', -s if s < 0 else s) - __math('log', absx) - r
    if isinf(r):
        raise 'OverflowError: math range error'
    return r

def _erf_series(x):
    # erf(x) = x*exp(-x*x)/sqrt(pi) * [2/1 + 4/3 x^2 + 8/15 x^4 + ...],
    # counted out from the small end; twenty-five terms settle every
    # |x| below one and a half.
    x2 = x * x
    acc = 0.0
    fk = 25.5
    for i in range(25):
        acc = 2.0 + x2 * acc / fk
        fk -= 1.0
    return acc * x * __math('exp', -x2) / _sqrtpi

def _erfc_contfrac(x):
    # erfc(x) = x*exp(-x*x)/sqrt(pi) * [1/(0.5 + x^2 -) 0.5/(2.5 + x^2 -)
    # 3.0/(4.5 + x^2 -) 7.5/(6.5 + x^2 -) ...]. The fraction is read from
    # its far end upward, where each step undoes the error of the one
    # below, so sixty terms answer to a couple of last places at any
    # width here. The square's own rounding error is carried apart and
    # handed to exp with it: at these widths exp feels that error as
    # whole last places of its own.
    if x >= 30.0:
        return 0.0
    x2 = x * x
    square_lo = __math('fma', x, x, -x2)
    e = __math('exp', -x2)
    if square_lo != 0:
        e = e * (1.0 - square_lo)
    t = 0.0
    k = 100
    while k >= 1:
        a = k * (k - 0.5)
        b = x2 + 2 * k + 0.5
        t = a / (b - t)
        k -= 1
    return x * e / (_sqrtpi * (x2 + 0.5 - t))

def erf(x):
    _check_real(x)
    _overflow_guard(x)
    x = float(x)
    if isnan(x):
        return x
    absx = -x if x < 0 else x
    if absx < 1.5:
        return _erf_series(x)
    cf = _erfc_contfrac(absx)
    if x > 0:
        return 1.0 - cf
    return cf - 1.0

def erfc(x):
    _check_real(x)
    _overflow_guard(x)
    x = float(x)
    if isnan(x):
        return x
    absx = -x if x < 0 else x
    if absx < 1.0:
        return 1.0 - _erf_series(x)
    cf = _erfc_contfrac(absx)
    if x > 0:
        return cf
    return 2.0 - cf








def nextafter(x, y, steps=1):
    if type(steps) != type(1) and type(steps) != type(True):
        raise 'TypeError: steps must be an integer'
    if steps < 0:
        raise 'ValueError: steps must be non-negative'
    for i in range(steps):
        x = __math('nextafter', x, y)
        if x == y:
            break
    return __math('fdiv', x, 1.0)

def ulp(x):
    return __math('ulp', x)

def remainder(x, y):
    _check_real(x)
    _check_real(y)
    if isnan(x) or isnan(y):
        return nan
    if isinf(x) or y == 0:
        raise 'ValueError: math domain error'
    if isinf(y):
        return __math('fdiv', x, 1.0)
    magnitude = fabs(y)
    residue = fmod(fabs(x), magnitude)
    other = magnitude - residue
    if residue > other:
        residue -= magnitude
    elif residue == other and fmod(fabs(x), 2 * magnitude) >= magnitude:
        residue -= magnitude
    if x < 0:
        residue = -residue
    if residue == 0:
        return copysign(0.0, x)
    return __math('fdiv', residue, 1.0)

def fmod(x, y):
    if isnan(x) or isnan(y):
        return nan
    if isinf(x) or y == 0:
        raise 'ValueError: math domain error'
    return __math('fmod', x, y)

def modf(x):
    _check_real(x)
    x = float(x)
    if isnan(x):
        return (nan, nan)
    if isinf(x):
        return (copysign(0.0, x), x)
    if x == 0:
        return (x, x)
    whole = trunc(x)
    fraction = x - whole
    return (fraction, __math('fdiv', whole, 1.0))

def frexp(x):
    ready = __math('frexp_plain', x)
    if ready is not None:
        return ready
    _check_real(x)
    if _is_integral(x):
        n = int(x)
        if n == 0:
            return (__math('fdiv', 0.0, 1.0), 0)
        mantissa, exponent = _int_frexp(-n if n < 0 else n)
        return (-mantissa if n < 0 else mantissa, exponent)
    x = float(x)
    return __math('frexp', x)

def ldexp(x, i):
    ready = __math('ldexp_plain', x, i)
    if ready is not None:
        return ready
    if type(i) != type(1) and type(i) != type(True):
        raise 'TypeError: ldexp exponent must be an integer'
    result = __math('ldexp', x, i)
    if isinf(result) and isfinite(x):
        raise 'OverflowError: math range error'
    return result


def exp2(x):
    return __math('pow', 2.0, x)

def fma(x, y, z):
    _check_real(x)
    _check_real(y)
    _check_real(z)
    return __math('fma', x, y, z)



def cbrt(x):
    _check_real(x)
    return __math('cbrt', x)




def sumprod(p, q, /):
    return __math('sumprod', p, q)
