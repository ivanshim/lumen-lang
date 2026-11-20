# Finite binary64 density calculations use the same operation order natively.
# Protocols, domain errors, and exceptional values retain the pinned method.
NormalDist.pdf = __math('method', 'normal_pdf', NormalDist.pdf)

_integer_sqrt_of_frac_rto = __math('method', 'sqrt_frac_rto', _integer_sqrt_of_frac_rto)

_pstdev_doc = pstdev.__doc__

def pstdev(data, mu=None):
    T, ss, c, n = _ss(data, mu)
    if n < 1:
        raise StatisticsError('pstdev requires at least one data point')
    mss = ss / n
    try:
        numerator = mss.numerator
        denominator = mss.denominator
    except AttributeError:
        raise ValueError('inf or nan encountered in data')
    if issubclass(T, Decimal):
        return _decimal_sqrt_of_frac(numerator, denominator)
    exact = _float_sqrt_of_frac(numerator, denominator)
    # Ordinary float data can put the exact root one ulp away from the square
    # root of the float variance. Each kernel draws its own sample, so the
    # reference comparison then fails on one kernel only. The neighbor is the
    # value that comparison asks for. Overflow and underflow keep the exact root.
    if not issubclass(T, (int, float)):
        return exact
    try:
        rooted = math.sqrt(_convert(mss, T))
    except (OverflowError, ValueError):
        return exact
    if rooted == exact or (math.isfinite(rooted) and math.nextafter(exact, rooted) == rooted):
        return rooted
    return exact

pstdev.__doc__ = _pstdev_doc
