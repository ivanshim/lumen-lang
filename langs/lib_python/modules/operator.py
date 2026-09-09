# These routines ask the ordinary operators to do their own work.
def add(a, b):
    return a + b

def sub(a, b):
    return a - b

def mul(a, b):
    return a * b

def truediv(a, b):
    return a / b

def floordiv(a, b):
    return a // b

def mod(a, b):
    return a % b

def pow(a, b):
    return a ** b

def neg(a):
    return -a

def pos(a):
    return +a

def abs(a):
    if a < 0:
        return -a
    return a

def eq(a, b):
    return a == b

def ne(a, b):
    return a != b

def lt(a, b):
    return a < b

def le(a, b):
    return a <= b

def gt(a, b):
    return a > b

def ge(a, b):
    return a >= b

def is_(a, b):
    return a is b

def is_not(a, b):
    return a is not b

def not_(a):
    return not a

def truth(a):
    return not not a

def contains(a, b):
    return b in a

def getitem(a, b):
    return a[b]

def index(a):
    if a != int(a):
        raise 'TypeError: value cannot be interpreted as an integer'
    return int(a)

class _ItemGetter:
    def __init__(self, names):
        self.names = names

    def take(self, value):
        if len(self.names) == 1:
            return value[self.names[0]]
        return [value[name] for name in self.names]

def itemgetter(*items):
    return _ItemGetter(items).take

class _AttrGetter:
    def __init__(self, names):
        self.names = names

    def take(self, value):
        result = []
        for name in self.names:
            # A dotted attribute is followed one word at a time.
            word = ''
            held = value
            for ch in name:
                if ch == '.':
                    held = getattr(held, word)
                    word = ''
                else:
                    word += ch
            result.append(getattr(held, word))
        if len(result) == 1:
            return result[0]
        return result

def attrgetter(*names):
    return _AttrGetter(names).take
