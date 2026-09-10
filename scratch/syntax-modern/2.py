a = [1, 2]
b = [3, 4]
def joined():
    return *a, 5
print(*joined())
for x in *a, *b:
    print(x)
print(*a, *b)
def given():
    yield *a,
idx = [0]
print(a[*idx])
