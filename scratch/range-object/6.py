back = reversed(range(3))
for v in back:
    print(v)
    break
print(list(back), list(back))
r = range(3)
print(back == back, back == reversed(r), r.index == r.index, r.index == range(3).index)
