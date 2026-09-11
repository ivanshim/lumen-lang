items = {}
for i in range(10**5):
    items[i] = i
result = 0
for i in range(10**5):
    result = result + items[i]
print(len(items), result)
