def value():
    print("once")
    return 7

x = 1
y = 2
z = 3
x = y = z = value()
print(x, y, z)

def local():
    a = b = c = 4
    return a + b + c

print(local())

def grammar_chain():
    x = y = z = 1, 2, 3

print("read")
