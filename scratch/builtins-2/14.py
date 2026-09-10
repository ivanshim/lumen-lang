names = sorted({'b': 1, 'a': 2})
print(names[0], 'a' in names, len(names), names == ['a', 'b'])
print(bool(sorted({})))
def empty():
    return dir()
print(empty())
