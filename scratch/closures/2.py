def adder(a):
    return lambda b: a + b
print(adder(2)(3))
def make():
    fs = [lambda: i for i in range(3)]
    return fs
fs = make()
print(fs[0](), fs[1](), fs[2]())
