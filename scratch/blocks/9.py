def a(): yield
def b(): x = yield
def c(): x = (yield 1)
def d(): yield 1, 2
def e(): yield from [1, 2]
def f(): print((yield 1))
print("read")
