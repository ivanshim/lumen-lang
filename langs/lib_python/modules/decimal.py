# The context may be held and changed. Decimal arithmetic awaits an
# exact-ratio constructor: the slash in this definition makes a real.
class Context:
    def __init__(self, prec=28):
        self.prec = prec

_context = Context()

def getcontext():
    return _context

def setcontext(context):
    global _context
    _context = context

class Decimal:
    def __init__(self, value='0', context=None):
        raise 'NotImplementedError: Decimal needs an exact rational constructor'
