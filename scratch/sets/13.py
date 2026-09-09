x = 90
print(sorted({i * j for i in range(4) if i > 0 for j in range(4) if j % 2}))
print(sorted({a + b for a, b in [[1, 2], [2, 3], [1, 2]]}))
print(sorted({x for x in range(4)}), x, sorted({i for i in []}))
def fresh():
    return {1}
a = fresh()
a.add(2)
print(sorted(a), sorted(fresh()))
print({1.0}, {-0.0}, {True}, set('aa'), len(set('ababa')))
print(sorted(set({1: 4, 2: 8})))
print(sorted({1}.copy()), sorted({1}.union()), sorted({1}.intersection()), sorted({1}.difference()))
print(sorted({True, False}), *{1, 2})
