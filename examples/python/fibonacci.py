def fib(n):
    a = 0
    b = 1
    for i in range(0, n):
        c = a + b
        a = b
        b = c
    return a

for i in range(0, 10):
    print(fib(i))
