class Q:
    def __init__(self, items): self.items = list(items)
    def __len__(self): return len(self.items)
class B:
    def __bool__(self): return False
def g(p, b):
    yield 'if' if p else 'else'
    yield 'ifb' if b else 'elseb'
    yield p and 'and'
    yield p or 'or'
    yield b and 'andb'
    yield b or 'orb'
    yield 'not' if not p else 'notnot'
    if p: yield 'stmt'
    elif b: yield 'stmtb'
    else: yield 'stmte'
    n = 0
    while p and n < 5:
        n += 1
        if n == 2: p.items.clear()
        yield n
def show(x):
    return x if isinstance(x, (str, int)) else type(x).__name__
print([show(x) for x in g(Q([]), B())], [show(x) for x in g(Q([1]), B())])
