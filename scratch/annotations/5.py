def key():
    print("key")
    return 0
values = [1]
values[key()]: Missing = 4
print(values)
values[key()]: 1 / 0
print(values)
[values][0]: Missing
print("read")
