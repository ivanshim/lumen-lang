class Walk:
    def __init__(self):
        self.n = 0
    def __iter__(self):
        return self
    def __next__(self):
        if self.n == 2:
            raise StopIteration
        self.n += 1
        return self.n
for value in Walk():
    print(value)
else:
    print("end")
