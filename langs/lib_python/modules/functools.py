# Wrapping keeps the function itself; metadata copying is a stub.
def _identity(function):
    return function

def wraps(wrapped, assigned=None, updated=None):
    return _identity

def reduce(function, sequence, *initial):
    seen = len(initial) != 0
    value = None
    if seen:
        value = initial[0]
    for item in sequence:
        if seen:
            value = function(value, item)
        else:
            value = item
            seen = True
    if not seen:
        raise 'TypeError: reduce() of empty iterable with no initial value'
    return value

class _Partial:
    def __init__(self, function, args, keywords):
        self.function = function
        self.args = args
        self.keywords = keywords

    def call(self, *args, **keywords):
        return self.function(*self.args, *args, **{**self.keywords, **keywords})

def partial(function, *args, **keywords):
    return _Partial(function, args, keywords).call
