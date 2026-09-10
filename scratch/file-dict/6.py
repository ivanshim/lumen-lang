print(7 & 3)
print(7 ^ 3)
print(~7)
print(1 << 3)
print(16 >> 2)
print(1 | 2 ^ 3 & 1)
print(1 << 2 + 1)
x = 7
x &= 3
x ^= 1
x <<= 2
x >>= 1
print(x)
# The dictionary-view expressions are read even before views can run.
def view_operations(d):
    return (d.keys() & d.keys(), d.items() ^ d.items())
