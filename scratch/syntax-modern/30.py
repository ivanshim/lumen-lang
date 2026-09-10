def make():
    value = 2
    def check():
        return value + 3, value < 4, 0 < value < 5
    def change():
        nonlocal value
        value += 4
    print(*check())
    change()
    print(*check())
    return check
print(*make()())
def build():
    rows = [[1]]
    alias = rows
    def read():
        return rows
    rows[0][0] = 2
    print(read(), alias)
    del rows
    rows = [[3]]
    return read, alias
read, alias = build()
print(read(), alias)
