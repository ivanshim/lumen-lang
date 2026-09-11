r = range(10, 0, -3)
print(r.start, r.stop, r.step, list(r), repr(r[1:5]), repr(r[::-1]))
print(range(2, 3, 4) == range(2, 7, 9), range(0) == range(0, 5, -1))
print(r.count(7), r.count(8), r.count(7.0), 7.0 in r, 7.5 in r)
f = r.index
print(f(4), min(r), max(r), list(range(0)))
values = []
for i in range(2, 5):
    values.append(i)
for i in r:
    values.append(i)
print(values)
