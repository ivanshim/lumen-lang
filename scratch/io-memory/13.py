import sys


# Check actual live holdings without requiring a particular object representation.
def check(value, other):
    before = sys.getrefcount(value)
    holds = [value, value, value]
    during = sys.getrefcount(value)
    holds.clear()
    after = sys.getrefcount(value)
    unrelated = [other, other, other]
    distinct = sys.getrefcount(value)
    print(repr(value), before > 0, during > before, after == before, distinct == before)
    assert before > 0
    assert during > before
    assert after == before
    assert distinct == before


check(True, 1)
check(False, 0)
check(True, False)
check(False, True)
check(None, False)
check(Ellipsis, None)
check(NotImplemented, Ellipsis)
check(37, True)
one, zero = 1, 0
print('identity', True is not one, False is not zero, True is not False, Ellipsis is not None)


# Staging a nested call must not retain another Python reference to its argument.
def equal_counts(before, after):
    assert before == after


before = sys.getrefcount(10)
equal_counts(before, sys.getrefcount(10))
print('call shapes stable')
