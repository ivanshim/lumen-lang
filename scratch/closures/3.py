def outer():
    x = 7
    def own():
        x = 9
        return x
    print(own(), x)
    def early():
        print(x)
        x = 10
    early()
outer()
