def make():
    n = 0
    def step():
        nonlocal n
        n += 1
        return n
    return step
s = make()
print(s(), s(), s())
