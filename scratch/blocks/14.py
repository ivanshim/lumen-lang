class Manager:
    def __init__(self, value):
        self.value = value
    def __enter__(self):
        return self.value
    def __exit__(self, kind, value, trace):
        return False

with Manager(5) as (x):
    print(x)
with Manager([1, [2, 3]]) as (a, (b, c)):
    print(a, b, c)
for x, y in [[1, 2], [3, 4]]:
    print(x, y)
else:
    print("done")
