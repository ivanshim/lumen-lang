def g(): yield from ()
def h(): x = yield from (1, 2)
def j(): yield 1, *[2, 3]
def k():
    def nested(): yield 1
    return 8
print(k())
