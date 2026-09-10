def value():
    print('once')
    return 42

a = b = c = value()
print(a)
print(b)
print(c)
