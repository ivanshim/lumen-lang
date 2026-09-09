def g(a, b=2, c=3):
    print(a, b, c)
def k(**kw):
    print(len(kw))
k(x=1, y=2)
args = [1, 2]
g(*args)
