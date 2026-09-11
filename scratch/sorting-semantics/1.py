calls = []
print(sorted([3, 1, 2], key=lambda v: (calls.append(v), -v)[1]), calls)
print(sorted("bca"), sorted({3: 0, 1: 0}), sorted(range(3), reverse=True))
