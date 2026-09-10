# Reals use the kernel's width; integer work stays exact.
pi = 3.141592653589793
e = 2.718281828459045
inf = __math('fdiv', 1.0, 0.0)
nan = __math('fdiv', 0.0, 0.0)

def sqrt(x):
    if x < 0:
        raise 'ValueError: math domain error'
    return __math('sqrt', x)

def fabs(x):
    if x == 0:
        return __math('fdiv', 0.0, 1.0)
    if x < 0:
        return __math('fdiv', -x, 1.0)
    return __math('fdiv', x, 1.0)

def floor(x):
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer floor'
    n = int(x)
    if n > x:
        n -= 1
    return n

def ceil(x):
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer ceiling'
    n = int(x)
    if n < x:
        n += 1
    return n

def trunc(x):
    if isinf(x) or isnan(x):
        raise 'ValueError: a non-finite value has no integer truncation'
    return int(x)

def pow(x, y):
    return __math('pow', x, y)

def exp(x):
    return __math('exp', x)

def log(x, base=None):
    if x <= 0:
        raise 'ValueError: math domain error'
    answer = __math('log', x)
    if base is not None:
        if base <= 0 or base == 1:
            raise 'ValueError: invalid logarithm base'
        answer = __math('fdiv', answer, __math('log', base))
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
    sign = __math('fdiv', 1.0, y)
    if y < 0 or sign < 0:
        return __math('fdiv', -fabs(x), 1.0)
    return fabs(x)

def gcd(*integers):
    result = 0
    for value in integers:
        if type(value) != type(1) and type(value) != type(True):
            raise 'TypeError: gcd needs integers'
        if value < 0:
            value = -value
        while value != 0:
            result, value = value, result % value
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
    # Compensation keeps small terms which a plain sum would lose.
    partials = []
    for x in values:
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
    return __math('fdiv', sum(partials), 1.0)

def prod(values, start=1):
    for x in values:
        start *= x
    return start

tau = 2 * pi

def comb(n, k):
    if type(n) != type(1) or type(k) != type(1):
        raise 'TypeError: comb needs integers'
    if n < 0 or k < 0:
        raise 'ValueError: comb arguments must be non-negative'
    if k > n:
        return 0
    k = min(k, n - k)
    result = 1
    for i in range(1, k + 1):
        result = result * (n - k + i) // i
    return result

def perm(n, k=None):
    if k is None:
        return factorial(n)
    if type(n) != type(1) or type(k) != type(1):
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
    if type(n) != type(1):
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
    if x <= 0:
        raise 'ValueError: math domain error'
    return __math('log2', x)

def log10(x):
    if x <= 0:
        raise 'ValueError: math domain error'
    return __math('log10', x)

def log1p(x):
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
    if x < -1 or x > 1:
        raise 'ValueError: math domain error'
    return __math('asin', x)

def acos(x):
    if x < -1 or x > 1:
        raise 'ValueError: math domain error'
    return __math('acos', x)

def atan(x):
    return __math('atan', x)

def atan2(y, x):
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
    if x < 1:
        raise 'ValueError: math domain error'
    return __math('acosh', x)

def atanh(x):
    if x <= -1 or x >= 1:
        raise 'ValueError: math domain error'
    return __math('atanh', x)

# These operations require binary representation or a special-function
# floor. Their names may be read, but no approximation is passed off.
def erf(x):
    raise 'NotImplementedError: erf is not supported'

def erfc(x):
    raise 'NotImplementedError: erfc is not supported'

def gamma(x):
    raise 'NotImplementedError: gamma is not supported'

def lgamma(x):
    raise 'NotImplementedError: lgamma is not supported'

def nextafter(x, y, steps=1):
    raise 'NotImplementedError: nextafter needs binary floating-point neighbours'

def ulp(x):
    raise 'NotImplementedError: ulp needs binary floating-point neighbours'

def remainder(x, y):
    raise 'NotImplementedError: remainder needs nearest-even binary division'

def fmod(x, y):
    raise 'NotImplementedError: fmod needs binary floating-point division'

def modf(x):
    raise 'NotImplementedError: modf needs tuple values'

def frexp(x):
    raise 'NotImplementedError: frexp needs tuple values'

def ldexp(x, i):
    if type(i) != type(1):
        raise 'TypeError: ldexp exponent must be an integer'
    return __math('fdiv', x * 2 ** i, 1.0)
