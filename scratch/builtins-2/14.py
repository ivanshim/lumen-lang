names = sorted({'b': 1, 'a': 2})
print(names[0], 'a' in names, len(names), names == ['a', 'b'])
if sorted({}):
    print(True)
else:
    print(False)
def empty():
    return dir()
print(empty())
