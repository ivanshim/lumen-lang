class I:
    def __index__(self):
        return 2
print([0, 1, 2, 3, 4][I():])
print(slice(I(), None).indices(5))
print("abc"[slice(0, 2)])
print(range(10)[slice(2, 8, 2)])
print(slice(1, 2) == slice(1, 2, None), slice(1) != slice(2))
print(hash(slice(1, 2, 3)) == slice(1, 2, 3).__hash__())
print(hash(slice(...)) == slice(...).__hash__())
