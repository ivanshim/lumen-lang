from collections import deque
def t(label, f):
    try:
        print(label, '->', repr(f()))
    except BaseException as e:
        print(label, '!!', type(e).__name__, e)
for src in ([], (), range(0), '', b'', deque()):
    d = deque([1, 2]); it = iter(d); rt = reversed(d)
    d.extend(src); d.extendleft(src); d += src
    t('empty %s' % type(src).__name__, lambda: (next(it), next(rt), list(it), list(rt)))
for meth in ('extend', 'extendleft'):
    d = deque([1, 2], 2); it = iter(d); rt = reversed(d)
    getattr(d, meth)([3])
    t('bounded %s fwd' % meth, lambda: next(it))
    t('bounded %s rev' % meth, lambda: next(rt))
d = deque([1]); it = iter(d); d.extend([5])
t('grown', lambda: next(it))
class Start:
    def __init__(self, d): self.d = d
    def __index__(self):
        self.d.append(2); return 0
d = deque([1]); t('start mutates', lambda: d.index(2, Start(d)))
class Stop:
    def __init__(self, d): self.d = d
    def __index__(self):
        self.d.append(2); return 10
d = deque([1]); t('stop mutates', lambda: d.index(2, 0, Stop(d)))
d = deque([1]); t('plain', lambda: (d.index(1), d.index(1, -1), d.index(1, 0, 5)))
