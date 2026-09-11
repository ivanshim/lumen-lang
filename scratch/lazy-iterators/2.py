class C:
    def __getitem__(self, i):
        if i == 3:
            raise IndexError
        return i

print(list(C()))
it = iter([1])
print(iter(it) is it, next(it), next(it, "end"))
print(list(iter(lambda: 1, 1)))
