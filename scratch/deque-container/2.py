class Q:
    def __init__(self, items): self.items = list(items)
    def __len__(self): return len(self.items)
class B:
    def __init__(self, items): self.items = list(items)
    def __bool__(self): return bool(self.items)
def run(label, f, *a):
    try:
        print(label, list(f(*a)))
    except Exception as e:
        print(label, 'ERR', type(e).__name__, e)
def gb(p):
    while bool(p):
        p.items.pop(0)
        yield 1
def gl(p):
    while len(p):
        p.items.pop(0)
        yield 1
def gn(p):
    while not not p:
        p.items.pop(0)
        yield 1
def gB(p):
    while p:
        p.items.pop(0)
        yield 1
def gi(p):
    if p:
        yield 'a'
    if p:
        yield 'b'
def gt(p):
    while True:
        if not p: break
        p.items.pop(0)
        yield 1
def gx(p):
    x = p
    while x:
        p.items.pop(0)
        yield 1
run('gb', gb, Q([1, 2, 3])); run('gl', gl, Q([1, 2, 3])); run('gn', gn, Q([1, 2, 3])); run('gB', gB, B([1, 2, 3])); run('gi', gi, Q([1])); run('gt', gt, Q([1, 2, 3])); run('gx', gx, Q([1, 2, 3]))
