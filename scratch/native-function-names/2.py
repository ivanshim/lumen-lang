def first():
    return 1

def second():
    return 2

initial = first.__code__
first.__code__ = second.__code__
name, qualified = first.__name__, first.__qualname__
assert name == 'first' and qualified == 'first'
first.__code__ = initial
assert first() == 1
assert first.__name__ is name and first.__qualname__ is qualified
first.__code__ = second.__code__
assert first() == 2
assert first.__name__ is name and first.__qualname__ is qualified
print('function name identity survives repeated code replacement')
