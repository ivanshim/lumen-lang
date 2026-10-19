import sys, cProfile

class Good:
    def __iter__(self):
        return iter((3, 5))

class One:
    def __iter__(self):
        return iter((8,))

class Bad:
    def __iter__(self):
        raise ValueError('iteration stopped')

def keywords():
    return {'default': 7}

def bad_keywords():
    raise ValueError('keywords stopped')

alias = max
bound = [].extend

def direct_good():
    return max(*Good())
def alias_good():
    return alias(*Good())
def method_good():
    return bound(*(Good(),))
def method_spread():
    return [].append(*Good())
def method_one():
    return [].append(*One())
def alias_method_one():
    f = [].append
    return f(*One())
def kw_nonmapping():
    return max(**Good())
def direct_bad():
    return max(*Bad())
def alias_bad():
    return alias(*Bad())
def method_bad():
    return [].append(*Bad())
def tail_good():
    f = max
    return f(*Good())
def tail_bad():
    f = max
    return f(*Bad())
def kw_good():
    return max((), **keywords())
def kw_alias():
    return alias((), **keywords())
def kw_bad():
    return max(**bad_keywords())
def kw_invalid_direct():
    return max(**{1: 2})
def kw_invalid_alias():
    return alias(**{1: 2})
def kw_invalid_tail():
    f = max
    return f(**{1: 2})
def kw_invalid_method():
    return [].append(**{1: 2})
def kw_invalid_bound():
    f = [].append
    return f(**{1: 2})
def kw_invalid_text():
    return ''.split(**{1: 2})
def kw_invalid_bool():
    return max(**{True: 2})
def kw_invalid_tuple():
    return max(**{(1,): 2})
def kw_invalid_duplicate():
    return max(**{1: 2}, **{1: 3})
def kw_duplicate():
    return max((), default=3, **{'default': 4})
def method_kw_duplicate():
    return ''.split(sep=' ', **{'sep': ','})

events = []
def hook(frame, event, arg):
    name = frame.f_code.co_name
    if event.startswith('c_'):
        name = getattr(arg, '__name__', '?')
    if name in ('__iter__', 'keywords', 'bad_keywords', 'max', 'append', 'extend', 'split'):
        events.append((event, name))

for case in (direct_good, alias_good, method_good, method_spread, method_one, alias_method_one,
             direct_bad, alias_bad, method_bad, tail_good, tail_bad,
             kw_good, kw_alias, kw_bad, kw_duplicate,
             method_kw_duplicate, kw_nonmapping, kw_invalid_direct, kw_invalid_alias,
             kw_invalid_tail, kw_invalid_method, kw_invalid_bound, kw_invalid_text,
             kw_invalid_bool, kw_invalid_tuple, kw_invalid_duplicate):
    events = []
    sys.setprofile(hook)
    try:
        result = case()
        outcome = ('ok', result)
    except (TypeError, ValueError) as error:
        outcome = ('error', type(error).__name__)
    sys.setprofile(None)
    print(case.__name__, outcome)
    print(events)

def run():
    return max(*Good())
p = cProfile.Profile()
p.runcall(run)
p.snapshot_stats()
for key in sorted(p.stats):
    if key[2] in ('__iter__', 'run') or 'builtins.max' in key[2]:
        cc, nc, tt, ct, callers = p.stats[key]
        print(key[2], nc, sorted((caller[2], values[0]) for caller, values in callers.items()))
