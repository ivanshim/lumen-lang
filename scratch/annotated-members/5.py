class C:
    x: Missing[One, Two] = 4
    x: Missing
    y: Missing = x + 1
    unused: Missing
print(C.x, C.y)
class D: x: Missing = 2; y: Missing = x + 1; z: Missing
print(D.x, D.y)
