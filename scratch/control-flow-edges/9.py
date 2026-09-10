a = [1, 2]
for x in a:
    print(x)
    if x == 1:
        a.append(3)
print(a)
b = [1, 2, 3, 4]
for x in b:
    print(x)
    if x == 1:
        del b[0]
print(b)
