# Bisection keeps the chosen half until no place lies between the bounds.
def bisect_left(a, x, lo=0, hi=None, key=None):
    if lo < 0:
        raise 'ValueError: lo must be non-negative'
    if hi is None:
        hi = len(a)
    while lo < hi:
        mid = (lo + hi) // 2
        value = a[mid] if key is None else key(a[mid])
        if value < x:
            lo = mid + 1
        else:
            hi = mid
    return lo

def bisect_right(a, x, lo=0, hi=None, key=None):
    if lo < 0:
        raise 'ValueError: lo must be non-negative'
    if hi is None:
        hi = len(a)
    while lo < hi:
        mid = (lo + hi) // 2
        value = a[mid] if key is None else key(a[mid])
        if x < value:
            hi = mid
        else:
            lo = mid + 1
    return lo

bisect = bisect_right

def insort_left(a, x, lo=0, hi=None, key=None):
    raise 'NotImplementedError: insort needs shared mutable list storage'

insort_right = insort_left
insort = insort_right
