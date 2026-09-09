a = [0, 1, 2, 3]
a[1:3]: Missing = [8, 9, 10]
print(a)
a[::2]: Missing = [4, 5, 6]
print(a)
def bound():
    print("bound")
    return 1
a[bound():3]: Missing
print(a)
def key():
    print("key")
    return 99
a[key()]: Missing
print("done")
