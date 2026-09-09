print(str(12), str(object="yes"), int(float("1e3")))
print(int(" -0x_fF ", base=0), int("101", 2), int("z", 36), int())
print(range(1, 10, 3)[2], range(9, 0, -2)[-1], len(range(4)))
for x in range(5, 0, -2):
    print(x, end=" ")
print()
print(*range(3), sep="-")
print(range(1, 10, 2))
