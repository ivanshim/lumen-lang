def bad():
    print("annotation ran")
    return 0

def f(a: bad(), b: Missing[One, Two] = 6) -> Missing[bad(), One | Two]:
    local: bad() = a + b
    unused: 1 / 0
    return local
print(f(5))
print(f(5, 7))
