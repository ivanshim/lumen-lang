print(hash(()), hash((0,)), hash((0, 0)))
print(hash((0.5,)), hash((0.5, (), (-2, 3, (4, 6)))))
print(hash((1, 2)) == hash((1.0, 2.0)))
try:
    print(hash([1]))
except TypeError as e:
    print(str(e))
try:
    print(hash((1, [2])))
except TypeError as e:
    print(str(e))
try:
    print({([1],): 2})
except TypeError as e:
    print(str(e))
