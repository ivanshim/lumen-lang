print(tuple(map(lambda a, b: a + b, [1, 2], [10, 20])))
print(sorted(iter([3, 1, 2])), sum(iter([1, 2, 3])))
print(min(iter([3, 1, 2])), max(iter([3, 1, 2])))
def pull(x):
    print("pull", x)
    return x
print(any(map(pull, [1, 2])), all(map(pull, [0, 1])))
it = iter([1, 2, 3])
print(2 in it, next(it))
print(",".join(map(str, [1, 2])))
m = map(str, [1, 2])
print(repr(m), next(m), list(m))
print(sorted(set(iter([3, 1, 3]))))
