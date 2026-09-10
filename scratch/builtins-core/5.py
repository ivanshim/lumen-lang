def twice(x):
    print(x)
    return x * 2
it = map(twice, [1, 2, 3])
print("ready")
print(next(it))
print(list(it))
it = iter([0, 1, 2])
print(any(it), next(it))
it = iter([1, 0, 2])
print(all(it), next(it))
it = iter([1, 2, 3, 4])
print(repr(list(zip(it, it))))
print(repr(list(filter(None, [0, 1, 2]))), repr(list(reversed("ab"))))
