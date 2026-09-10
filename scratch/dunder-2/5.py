import operator
import math
import sys
class N:
    def __index__(self):
        return 2
    def __int__(self):
        return 3
    def __float__(self):
        return 1.5
    def __round__(self, digits=0):
        return digits + 10
    def __trunc__(self):
        return 4
    def __floor__(self):
        return 5
    def __ceil__(self):
        return 6
    def __sizeof__(self):
        return 20
    def __divmod__(self, other):
        return 7
    def __rdivmod__(self, other):
        return 8
    def __reversed__(self):
        return iter([3, 2, 1])
n = N()
print(int(n), float(n), round(n), round(n, 2))
print(operator.index(n), bin(n), hex(n), oct(n))
print(math.trunc(n), math.floor(n), math.ceil(n))
print(divmod(n, 2), divmod(2, n))
print(list(reversed(n)))
print([10, 20, 30, 40][n:])
