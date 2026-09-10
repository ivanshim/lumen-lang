def acc():
    total = 0
    while True:
        v = yield total
        total += v
it = acc()
print(next(it))
print(it.send(5))
print(it.send(7))
print(it.close())
print(next(it, "ended"))
