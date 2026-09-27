# The Lumen library file langs/lib_lumen/round.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def round(x, decimals=None):
    if decimals is None:
        if isinstance(x, float):
            if x != x:
                raise ValueError('cannot convert float NaN to integer')
            if x == float('inf') or x == -float('inf'):
                raise OverflowError('cannot convert float infinity to integer')
        decimals = 0
    if not isinstance(decimals, int):
        raise TypeError('ndigits must be an integer')
    scale = 1
    i = 0
    while i < decimals:
        scale = scale * 10
        i = i + 1
    y = x * scale
    if y >= 0:
        r = (y * 2 + 1) // 2
    else:
        r = (y * 2 - 1) // 2
    q = r // scale
    if q * scale == r:
        return q
    else:
        return r / scale
