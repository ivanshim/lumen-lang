def add(a, b): return a + b
def union(a, b): return a - b
s = {1}
s.add(add(1, 1))
print(add(2, 3), union(8, 3), sorted(s.union([3])))
