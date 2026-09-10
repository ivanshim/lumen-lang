def slices(a, obj):
    a[:] *= 2
    obj.items[1::2] += [1]
    a[1:2, ..., ::-1] = [2]
print("read")
