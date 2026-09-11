def pull(x):
    print("pull", x)
    return x

left = filter(None, map(pull, [1, 2, 3]))
right = map(pull, [10, 20, 30])
for a, b in zip(left, right):
    print(a, b)
    break
e = enumerate("ab", 1)
print(list(e))
print(list(e))
