def make():
    rows = [None, None]
    for n in range(2):
        rows[n] = [lambda: i for i in range(n + 1)]
    return rows
rows = make()
print(rows[0][0](), rows[1][0]())
rows = [[lambda: x for x in range(y + 1)] for y in range(2)]
print(rows[0][0](), rows[1][0]())
def writes():
    fs = [lambda: y for x in range(3) if (y := x) >= 0]
    return fs
fs = writes()
print(fs[0](), fs[2]())
