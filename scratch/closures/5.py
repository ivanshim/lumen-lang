def outer():
    x = 4
    def local():
        x = 9
        return x
    print(local(), x)
    fs = []
    for i in range(3):
        fs = fs + [lambda: i]
    print(fs[0](), fs[1](), fs[2]())
    ds = [lambda i=i: i for i in range(3)]
    print(ds[0](), ds[1](), ds[2]())
outer()
