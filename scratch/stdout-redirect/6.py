import sys, io
first = io.StringIO()
second = io.StringIO()
class Switch:
    def __str__(self):
        sys.stdout = second
        return 'held'
sys.stdout = first
print(Switch())
sys.stdout = None
print(Switch(), file=None, flush=True)
print_was_silent = sys.stdout is None
sys.stdout = sys.__stdout__
print(repr(first.getvalue()), repr(second.getvalue()), print_was_silent)
class NoFlush:
    def write(self, text):
        pass
    def flush(self):
        raise ValueError('flush failed')
try:
    print('x', file=NoFlush(), flush=True)
except ValueError as error:
    print(str(error))
