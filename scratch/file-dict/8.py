values = {1: 2}, {3: 4}
print(len(values))
print(values[1][3])
def pair():
    return {1: 2}, {3: 4}
print(pair()[0][1])
def single():
    return 9,
print(len(single()))
count = 0
for d in {1: 2}, {3: 4}:
    count += len(d)
print(count)
values = 1, *range(2, 4)
print(sum(values))
print(len((1, 2)), len((3, 4)))
