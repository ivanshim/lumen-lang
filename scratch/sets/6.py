s = set()
print(s, len(s), not s)
s.update([3, 1, 3], {2, 4})
t = s
c = s.copy()
s.add(5)
print(sorted(t), sorted(c), t is s, c is s)
s |= {6}
s &= {1, 3, 5, 6}
s -= {3}
s ^= {5, 7}
print(sorted(s), sorted(t))
print(sorted(s.union([8], [9])), sorted(s.intersection([1, 6], [6, 9])))
print(sorted(s.difference([1], [7])), sorted(s.symmetric_difference([6, 8])))
print(s.issubset([1, 6, 7, 8]), s.issuperset([1]), s.isdisjoint([2, 3]))
print({1} < s, s > {1}, s >= {1, 6, 7}, s <= {1, 6, 7}, s != {1})
print(sorted(frozenset([2, 1, 2])), sorted({*[1, 2], *[2, 3]}))
walk = []
for x in s:
    walk.append(x)
print(sorted(walk))
s.intersection_update([1, 7])
s.difference_update([1])
s.symmetric_difference_update([7, 9])
print(sorted(t), s.pop(), s)
c.clear()
print(c, len(c), 9 not in c)
print(len({True, 1, 1.0, False, 0}), {None}, {'x'}, {} == set())
