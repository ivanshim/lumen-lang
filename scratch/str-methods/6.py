def order(x):
    return -x
xs = [2, 3, 1, 2]
xs.sort(key=order, reverse=True)
print(xs, sorted(xs, key=order))
fn = "abc".upper
print(fn(), "a,b,c".split(maxsplit=1, sep=","))
