x = 100
def outer(a):
    def middle():
        def inner():
            nonlocal a
            a += 2
            return a
        return inner
    return middle
m = outer(3)
s = m()
print(s(), s(), m()())
def make():
    x = 7
    def put():
        global x
        x = 11
    put()
    return x
print(make(), x)
def later():
    def read():
        return x
    x = 12
    return read
print(later()())
def recursive():
    def fact(n):
        if n < 2:
            return 1
        return n * fact(n - 1)
    return fact
print(recursive()(5))
