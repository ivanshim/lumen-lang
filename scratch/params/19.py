def positional():
    print("positional")
    return [1]
def keyword():
    print("keyword")
    return 2
def f(a, b):
    print(a, b)
f(b=keyword(), *positional())
