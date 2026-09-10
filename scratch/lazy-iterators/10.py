class Reverse:
    def __reversed__(self):
        return iter([3, 2, 1])
print(list(reversed(Reverse())))
print(list(reversed(range(1, 7, 2))))
x = [1]
it = iter(x)
x.append(2)
print(list(it), list(it))
