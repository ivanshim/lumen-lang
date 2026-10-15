import pickle
from collections import deque
def t(label, f):
    try:
        print(label, '->', repr(f()))
    except BaseException as e:
        print(label, '!!', type(e).__name__, e)
def red(it):
    factory, args = it.__reduce__()
    return factory.__name__, args[1], list(factory(*args))
for name, make in (('fwd', iter), ('rev', reversed)):
    for change in ('none', 'grow', 'shrink', 'grow2'):
        d = deque([1, 2, 3, 4]); it = make(d); next(it)
        if change == 'grow': d.append(9)
        if change == 'shrink': d.pop()
        if change == 'grow2': d.appendleft(0); d.append(9)
        t(name + ' ' + change, lambda: red(it))
    d = deque([1, 2, 3]); it = make(d); next(it); d.append(9)
    t(name + ' detect', lambda: next(it))
    t(name + ' after detect', lambda: red(it))
    d.append(7)
    t(name + ' after detect grow', lambda: red(it))
    d = deque([1, 2, 3]); it = make(d); next(it); d.append(9)
    t(name + ' pickle grown', lambda: list(pickle.loads(pickle.dumps(it))))
    d = deque([1, 2, 3]); it = make(d); next(it); d.pop()
    t(name + ' pickle shrunk', lambda: list(pickle.loads(pickle.dumps(it))))
    for proto in range(pickle.HIGHEST_PROTOCOL + 1):
        d = deque([1, 2, 3]); it = make(d); next(it)
        t(name + ' pickle %d' % proto, lambda: list(pickle.loads(pickle.dumps(it, proto))))
