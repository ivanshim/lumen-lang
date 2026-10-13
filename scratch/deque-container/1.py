from collections import deque
import itertools, operator

def t(label, f):
    try:
        print(label, '->', repr(f()))
    except BaseException as e:
        print(label, '!!', type(e).__name__, e)

class Eq:
    log = []
    def __init__(self, n): self.n = n
    def __eq__(self, o):
        Eq.log.append((self.n, getattr(o, 'n', o)))
        return getattr(o, 'n', o) == self.n
    __hash__ = None
    def __repr__(self): return 'Eq(%d)' % self.n
class Boom:
    def __eq__(self, o): raise KeyError('boom')
class Mut:
    def __init__(self, q, r, how): self.q, self.r, self.how = q, r, how
    def __repr__(self): return 'Mut'
    def __eq__(self, o):
        getattr(self.q, self.how[0])(*self.how[1:]); return self.r
    __hash__ = None
for how in [('clear',), ('append', 9), ('pop',), ('rotate', 1), ('popleft',), ('appendleft', 4)]:
    for meth in ['remove', 'index', 'count', '__contains__']:
        for res in (False, True):
            q = deque([1, 2, 3, 4])
            q[1] = Mut(q, res, how)
            t('%s %s %s' % (meth, how, res), lambda: (getattr(q, meth)(7), list(q)))
for src in [[1, 2, 3], (4, 5), range(3), 'xy', b'ab', iter([7, 8]), deque([1, 2]), {1: 2}, {5}]:
    for ml in (None, 0, 1, 3, 10):
        for name in ('extend', 'extendleft', 'append'):
            q = deque([0, 9], maxlen=ml) if ml != 0 else deque(maxlen=0)
            t('%s %r %s' % (name, src if not hasattr(src, '__next__') else 'iter', ml),
              lambda: (getattr(q, name)(src), list(q)) if name != 'append' else (q.append(src), len(q)))
q = deque([1, 2]); t('self', lambda: (q.extend(q), q.extendleft(q), list(q)))
q = deque([1, 2], 3); t('selfm', lambda: (q.extend(q), list(q)))
def gen():
    yield 1
    raise ValueError('mid')
q = deque([5]); t('gen raise', lambda: q.extend(gen())); print(list(q))
q = deque([5]); t('gen raise l', lambda: q.extendleft(gen())); print(list(q))
q = deque(range(3)); t('gen mutate', lambda: q.extend(x for x in q)); 
t('cmp plain', lambda: [deque([1,2,3]) < deque([1,2,4]), deque([1,2]) < deque([1,2,3]), deque([1,2,3]) >= deque([1,2,3]), deque('ab') == deque('ab'), deque('ab') != deque('ab'), deque([float('nan')]) == deque([float('nan')])])
nan = float('nan'); t('cmp nan', lambda: (deque([nan]) == deque([nan]), nan in deque([nan]), deque([nan]).count(nan), deque([nan]).index(nan)))
t('cmp mixed', lambda: deque([1]) < deque(['a']))
t('cmp user', lambda: (deque([Eq(1), Eq(2)]) == deque([Eq(1), Eq(2)]), Eq.log))
t('cmp boom', lambda: deque([Boom()]) == deque([Boom()]))
t('contains boom', lambda: 1 in deque([1, Boom()]))
t('contains boom2', lambda: 2 in deque([1, Boom()]))
t('rem boom', lambda: deque([Boom()]).remove(3))
t('count boom', lambda: deque([Boom()]).count(3))
t('idx boom', lambda: deque([Boom()]).index(3))
t('index slices', lambda: [deque('abcabc').index('b', s, e) if True else 0 for s, e in [(0, 6), (2, 6), (-3, 6), (0, -1)]])
t('index err', lambda: deque('abc').index('c', 0, 2))
t('index idx', lambda: deque('abc').index('c', 2**70, -2**70))
t('index __index__', lambda: deque('abc').index('c', True, 3.0))
t('index mixed', lambda: (deque([1, 'a', 2.0, None, True]).index(True), deque([1, 'a', 2.0, None]).index(2), 2 in deque([1, 2.0]), deque([1, 1.0, True]).count(1)))
t('rotate', lambda: [(lambda q: (q.rotate(n), list(q)))(deque('abcde')) for n in (0, 1, 2, 5, 6, -1, -7, 100)])
t('maxlen rotate', lambda: (lambda q: (q.rotate(2), q.append(9), q.appendleft(8), list(q)))(deque('abc', 4)))
t('slice assign', lambda: (lambda q: (q.__setitem__(1, 'z'), q.__setitem__(-1, 'y'), list(q)))(deque('abc')))
t('setitem idx', lambda: deque('abc').__setitem__(slice(0, 1), 1))
t('getitem slice', lambda: deque('abc')[1:2])
t('getitem bool', lambda: (deque('abc')[True], deque('abc')[-3]))
t('delitem', lambda: (lambda q: (q.__delitem__(0), q.__delitem__(-1), list(q)))(deque('abc')))
t('iter len', lambda: [len(deque(range(n))) for n in range(4)])
it = iter(deque('abcd')); next(it)
t('iter hint', lambda: (it.__length_hint__(), list(it), it.__length_hint__()))
it = reversed(deque('abcd')); next(it)
t('riter hint', lambda: (it.__length_hint__(), list(it), it.__length_hint__()))
q = deque('abcd'); it = iter(q); next(it); q.append(1)
t('iter mutated', lambda: next(it)); t('iter after', lambda: (next(it, 'end'), it.__length_hint__()))
q = deque('abcd'); it = reversed(q); next(it); q.append(1)
t('riter mutated', lambda: next(it)); t('riter after', lambda: (next(it, 'end')))
q = deque('abcd'); it = reversed(q); list(it); q.append(1)
t('riter exhausted then mutated', lambda: next(it, 'end'))
q = deque('abcd'); it = iter(q); list(it); q.append(1)
t('iter exhausted then mutated', lambda: next(it, 'end'))
t('iter setitem', lambda: (lambda q: (lambda i: (q.__setitem__(1, 'Z'), list(i)))(iter(q)))(deque('abc')))
t('mul', lambda: [list(deque('ab') * n) for n in (-1, 0, 1, 2, 3)] + [list((deque('ab', 3) * 5)), list(deque('a', 3) * 5)])
t('imul', lambda: (lambda q: (q.__imul__(3), list(q)))(deque('ab', 5)))
t('imul huge', lambda: deque('ab') * (2**62))
t('imul huge2', lambda: deque('ab') * 2**70)
t('add max', lambda: list(deque('ab', 3) + deque('cde')))
t('iadd', lambda: (lambda q: (q.__iadd__('xyz'), list(q)))(deque('ab', 4)))
t('copy', lambda: (lambda q: (q.copy() == q, q.copy() is q, q.copy().maxlen))(deque('ab', 4)))
class Sub(deque):
    def __init__(self, *a, **k): super().__init__(*a, **k); self.extra = 1
t('sub copy', lambda: (type(Sub('ab').copy()), type(Sub('ab').__copy__()), Sub('ab').copy().extra))
class Bad(deque):
    def __new__(cls, *a): return 5
t('bad new', lambda: Bad())
t('bool', lambda: [bool(deque(x)) for x in ([], [0])])
t('init twice', lambda: (lambda q: (q.__init__('xy', 3), list(q), q.maxlen))(deque('abcdef', 7)))
t('init noarg', lambda: (lambda q: (q.__init__(), list(q), q.maxlen))(deque('abcdef', 7)))
t('maxlen big', lambda: deque(maxlen=2**62).maxlen)
t('maxlen huge', lambda: deque(maxlen=2**70))
t('maxlen None', lambda: deque([1, 2, 3], None).maxlen)
t('maxlen bool', lambda: deque([1, 2, 3], True))
t('maxlen index', lambda: deque([1, 2, 3], maxlen=2.0))
t('insert idx', lambda: deque().insert('a', 1))
t('insert many', lambda: [(lambda q: (q.insert(i, 'Z'), ''.join(q)))(deque('abc')) for i in (-9, -3, -1, 0, 1, 3, 9)])
t('rotate idx', lambda: deque('abc').rotate('1'))
t('rotate big', lambda: deque('abc').rotate(2**70))
t('reverse', lambda: (lambda q: (q.reverse(), list(q)))(deque('abc')))
t('pickle', lambda: __import__('pickle').loads(__import__('pickle').dumps(deque([1, [2], 'x'], 5))))
t('str', lambda: (str(deque('ab')), repr(deque([deque()]))))
t('sorted', lambda: sorted(deque([3, 1, 2])))
t('list()', lambda: (list(deque('abc')), tuple(deque('abc')), set(deque('abc')) == set('abc'), ''.join(deque('abc')), sum(deque([1, 2]))))
t('unpack', lambda: (lambda a, b, c: (a, b, c))(*deque('abc')))
t('eq other', lambda: (deque('a') == 'a', deque('a') == ['a'], deque() == deque()))
t('lt other', lambda: deque('a') < ['a'])
t('del slice', lambda: deque('abc').__delitem__(slice(1)))
t('index 2args', lambda: deque('abc').index('b', 1))
t('mul str', lambda: deque('ab') * 'a')
t('mul float', lambda: deque('ab') * 2.0)
t('index None', lambda: deque('ab').index('a', None))
t('index obj', lambda: deque('ab').index('a', object()))
class Ix:
    def __index__(self): return 1
t('index Ix', lambda: deque('ab').index('b', Ix()))
t('mul Ix', lambda: deque('ab') * Ix())
