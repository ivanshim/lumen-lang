value = 7
def f(a=value):
    print(a)
value = 9
f()
f(3)
f()
def make(x):
    def inner(a=x):
        return a
    return inner
first = make(4)
second = make(5)
print(first(), second(), first())
def default():
    print("default")
    return 8
def g(x=default()):
    print(x)
g()
g()
