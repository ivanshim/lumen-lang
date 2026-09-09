n = 5
def g(a, b=n * 2, c=None):
    print(a, b, c)
n = 0
g(1)
g(1, 2)
g(1, c=3)
