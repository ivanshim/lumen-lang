empty = None
print(empty is None, empty is not None)
print(True is False, True is not None)
one = [1]
alias = one
other = [1]
print(one is alias, one is other, one == other)
def first():
    print("first")
    return one
def second():
    print("second")
    return one
print(first() is second())
print(not empty is None, empty is not False)
if empty is None:
    print("singleton")
