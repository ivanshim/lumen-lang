class Counter:
    def __init__(self):
        self.i = 0
    def __iter__(self):
        return self
    def __next__(self):
        if self.i == 3:
            raise StopIteration
        self.i += 1
        return self.i
c = Counter()
print(iter(c) is c)
for item in c:
    print(item)
print(next(c, "done"))
