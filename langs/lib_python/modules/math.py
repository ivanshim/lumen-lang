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
    if x < 0:
        return -float(x)
    return float(x)

def floor(x):
    n = int(x)
    if n > x:
        n -= 1
    return n

def ceil(x):
    n = int(x)
    if n < x:
        n += 1
    return n

def trunc(x):
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
        answer /= __math('log', base)
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
        return -fabs(x)
    return fabs(x)

def gcd(*integers):
    result = 0
    for value in integers:
        if value < 0:
            value = -value
        while value != 0:
            result, value = value, result % value
    return result

def factorial(n):
    if n < 0:
        raise 'ValueError: factorial() not defined for negative values'
    if n != int(n):
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
    return float(sum(partials))

def prod(values, start=1):
    for x in values:
        start *= x
    return start
