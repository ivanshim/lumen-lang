calls = []
def key(v):
    calls.append(v)
    return 0
print(min([2.0, 2, 3], key=key), calls)
print(max([2.0, 2], key=lambda v: 0), min(3, 1, 2), max("Za"))
print("Z" < "a", (1, 2) < (1, 3), [1, 2] < [1, 3])
