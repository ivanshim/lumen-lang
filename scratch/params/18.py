def f(a,
      /,
      b=2,
      *rest,
      c=3,
      **kw):
    print(a, b, rest, c, len(kw))
f(
  1,
  *[4, 5],
  **{"c": 6, "x": 7},
)
def g(x=2, *, y=3): print(x, y)
g(y=4)
a = []
a.append(*[9])
print(a)
