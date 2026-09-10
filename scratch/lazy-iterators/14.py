class Box:
    def __init__(self, value):
        self.value = value

boxes = map(Box, [4, 5])
print(next(boxes).value, next(boxes).value)
d = {"a": 1, "b": 2}
print(list(reversed(d)), list(reversed(d.values())))
def stop(x):
    raise StopIteration
f = filter(stop, [1, 2])
print(list(f), list(f))
