pairs = [(1, "b"), (0, "a"), (1, "a")]
pairs.sort(key=lambda p: p[0])
print(pairs, sorted(pairs, key=lambda p: p[0], reverse=True))
