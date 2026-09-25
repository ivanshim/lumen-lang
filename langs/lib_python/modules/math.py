# Reals use the kernel's width; integer work stays exact.
pi = 3.141592653589793
e = 2.718281828459045
inf = __math('fdiv', 1.0, 0.0)
nan = __math('fdiv', 0.0, 0.0)

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
        raise 'ValueError: math domain error'
    return __math('sqrt', x)

def fabs(x):
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
    return hasattr(x, name)

def floor(x):
    if _answers_own(x, '__floor__'):
        return x.__floor__()
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer floor'
    n = int(x)
    if n > x:
        n -= 1
    return n

def ceil(x):
    if _answers_own(x, '__ceil__'):
        return x.__ceil__()
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer ceiling'
    n = int(x)
    if n < x:
        n += 1
    return n

def trunc(x):
    if _answers_own(x, '__trunc__'):
        return x.__trunc__()
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
    e = n.bit_length() - 1
    return n / (1 << e), e

def _log_int(xi, working, per_bit):
    if xi <= 0:
        raise 'ValueError: math domain error'
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
            raise 'ValueError: math domain error'
        return __math(working, real)
    xi = _index_or_none(x)
    if xi is not None:
        return _log_int(xi, working, per_bit)
    if x <= 0:
        raise 'ValueError: math domain error'
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
    if rel_tol < 0 or abs_tol < 0:
        raise 'ValueError: tolerances must be non-negative'
    if a == b:
        return True
    if isinf(a) or isinf(b):
        return False
    difference = fabs(a - b)
    return difference <= abs_tol or difference <= rel_tol * fabs(a) or difference <= rel_tol * fabs(b)

def copysign(x, y):
    _check_real(x)
    _check_real(y)
    sign = __math('fdiv', 1.0, y)
    if y < 0 or sign < 0:
        return __math('fdiv', -fabs(x), 1.0)
    return fabs(x)

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
    if n < 0:
        raise 'ValueError: factorial() not defined for negative values'
    if type(n) != type(1) and type(n) != type(True):
        raise 'TypeError: factorial needs an integer'
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
        _check_real(x)
        if isnan(x):
            saw_nan = True
            continue
        if x == inf:
            pos_inf = True
            continue
        if x == -inf:
            neg_inf = True
            continue
        kept = []
        for y in partials:
            if fabs(x) < fabs(y):
                x, y = y, x
            high = x + y
            low = y - (high - x)
            if low != 0:
                kept.append(low)
            x = high
        kept.append(x)
        partials = kept
    if saw_nan:
        return nan
    if pos_inf and neg_inf:
        raise 'ValueError: -inf + inf in fsum'
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

def prod(values, *, start=1):
    for x in values:
        start *= x
    return start

tau = 2 * pi

def comb(n, k):
    if (type(n) != type(1) and type(n) != type(True)) or (type(k) != type(1) and type(k) != type(True)):
        raise 'TypeError: comb needs integers'
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
    if (type(n) != type(1) and type(n) != type(True)) or (type(k) != type(1) and type(k) != type(True)):
        raise 'TypeError: perm needs integers'
    if n < 0 or k < 0:
        raise 'ValueError: perm arguments must be non-negative'
    if k > n:
        return 0
    result = 1
    for i in range(n - k + 1, n + 1):
        result *= i
    return result

def isqrt(n):
    if type(n) != type(1) and type(n) != type(True):
        raise 'TypeError: isqrt needs an integer'
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
    result = 0.0
    for value in coordinates:
        result = __math('hypot', result, value)
    return result

def dist(p, q):
    p, q = list(p), list(q)
    if len(p) != len(q):
        raise 'ValueError: both points must have the same number of dimensions'
    return hypot(*[p[i] - q[i] for i in range(len(p))])

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
        raise 'ValueError: math domain error'
    return __math('atanh', x)

# These operations require binary representation or a special-function
# floor. Their names may be read, but no approximation is passed off.
def erf(x):
    _check_real(x)
    raise 'NotImplementedError: erf is not supported'

def erfc(x):
    _check_real(x)
    raise 'NotImplementedError: erfc is not supported'

def gamma(x):
    _check_real(x)
    raise 'NotImplementedError: gamma is not supported'

def lgamma(x):
    _check_real(x)
    raise 'NotImplementedError: lgamma is not supported'

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
    raise 'NotImplementedError: modf needs tuple values'

def frexp(x):
    raise 'NotImplementedError: frexp needs tuple values'

def ldexp(x, i):
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

def fmin(x, y):
    _check_real(x)
    _check_real(y)
    return __math('fmin', x, y)

def fmax(x, y):
    _check_real(x)
    _check_real(y)
    return __math('fmax', x, y)

def cbrt(x):
    _check_real(x)
    return __math('cbrt', x)

def signbit(x):
    _check_real(x)
    return __math('signbit', x) != 0.0

def isnormal(x):
    _check_real(x)
    return __math('isnormal', x) != 0.0

def issubnormal(x):
    _check_real(x)
    return __math('issubnormal', x) != 0.0

def sumprod(p, q):
    total = 0
    for p_i, q_i in zip(p, q, strict=True):
        total = total + p_i * q_i
    return total
