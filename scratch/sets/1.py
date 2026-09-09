a = set([1, 2]); a.add(3); a.discard(9); print(sorted(a | {4}), sorted(a & {2, 3}), sorted(a - {1}), sorted(a ^ {1, 9}))
