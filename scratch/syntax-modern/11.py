a = [1, 2]
b = [3, 4]
for x in *a, *b:
    print(x)
for x in 5, 6:
    print(x)
print(*a, *b)
