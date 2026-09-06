print(1 + 2 * 3)

x = 0
y = 5

if x < y and y == 5:
    print(100)
elif x == y:
    print(150)
else:
    print(200)

i = 0
total = 0

while i < 10:
    if i == 5:
        i = i + 1
        continue

    if i == 8:
        break

    total = total + i
    print(total)
    i = i + 1

print(total)
print(True)
print(False)
print(not False)
print(-10 + 3)
print(0x1F)
print("done", "with", 3, "values")
