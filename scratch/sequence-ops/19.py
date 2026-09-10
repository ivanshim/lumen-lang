try:
    print([] in {})
except TypeError as e:
    print(str(e))
try:
    print([] in {1})
except TypeError as e:
    print(str(e))
print([1, 2] in [[1, 2]])
try:
    print('abc'[1.5])
except TypeError as e:
    print(str(e))
