# The names a program reaches without naming a module at all.
#
# CPython keeps these in the very dictionary that an unqualified name is
# looked up in, so writing a new value onto the module changes what
# every program run afterwards sees. Here the module is an ordinary one
# standing beside that dictionary rather than being it, so writing onto
# it changes what builtins.x answers and nothing else: a program that
# rebinds builtins.len and then writes len still gets the real one.
#
# Each name below is bound to the builtin of the same name, so what the
# module hands out is the thing itself and not a copy that behaves like
# it. The names CPython has and this runtime does not are simply absent,
# so hasattr says no for them; they are listed at the foot of the file.

ArithmeticError = ArithmeticError
AssertionError = AssertionError
AttributeError = AttributeError
BaseException = BaseException
BaseExceptionGroup = BaseExceptionGroup
EOFError = EOFError
Exception = Exception
ExceptionGroup = ExceptionGroup
ImportError = ImportError
IndexError = IndexError
KeyError = KeyError
KeyboardInterrupt = KeyboardInterrupt
LookupError = LookupError
NameError = NameError
NotImplementedError = NotImplementedError
OSError = OSError
OverflowError = OverflowError
RecursionError = RecursionError
RuntimeError = RuntimeError
StopIteration = StopIteration
SyntaxError = SyntaxError
SystemExit = SystemExit
TypeError = TypeError
UnboundLocalError = UnboundLocalError
UnicodeError = UnicodeError
ValueError = ValueError
ZeroDivisionError = ZeroDivisionError

Warning = Warning
BytesWarning = BytesWarning
DeprecationWarning = DeprecationWarning
EncodingWarning = EncodingWarning
FutureWarning = FutureWarning
ImportWarning = ImportWarning
PendingDeprecationWarning = PendingDeprecationWarning
ResourceWarning = ResourceWarning
RuntimeWarning = RuntimeWarning
SyntaxWarning = SyntaxWarning
UnicodeWarning = UnicodeWarning
UserWarning = UserWarning

# A module's names do not fall back on the reader's word for the root
# class, so object reads as nothing inside one. An empty class stands on
# the root all the same, and the list of bases it was built on hands the
# real class over.
class _Root:
    pass


bool = bool
classmethod = classmethod
complex = complex
dict = dict
enumerate = enumerate
filter = filter
float = float
frozenset = frozenset
int = int
list = list
map = map
object = _Root.__bases__[0]
property = property
range = range
reversed = reversed
set = set
slice = slice
staticmethod = staticmethod
str = str
super = super
tuple = tuple
type = type
zip = zip

__import__ = __import__
abs = abs
all = all
any = any
bin = bin
callable = callable
chr = chr
compile = compile
delattr = delattr
dir = dir
divmod = divmod
eval = eval
exec = exec
format = format
getattr = getattr
globals = globals
hasattr = hasattr
hash = hash
hex = hex
id = id
input = input
isinstance = isinstance
issubclass = issubclass
iter = iter
len = len
locals = locals
max = max
min = min
next = next
oct = oct
ord = ord
pow = pow
print = print
repr = repr
round = round
setattr = setattr
sorted = sorted
sum = sum
vars = vars

# The reader seeds __debug__ into the outermost names of the program
# rather than into a module's, so it is read from there. There is no
# switch in this runtime that turns the checks off, so it is always
# true.
__debug__ = globals()['__debug__']

# True, False, None, Ellipsis, bytes and bytearray are words the reader
# knows rather than names it can be asked for, so none of them can stand
# on the left of an assignment. The module asks the loader for itself
# and writes them on from outside instead, which leaves them under the
# names a program asks this module for.
_self = __load_module('builtins')
setattr(_self, 'True', True)
setattr(_self, 'False', False)
setattr(_self, 'None', None)
setattr(_self, 'Ellipsis', ...)
setattr(_self, 'bytes', bytes)
setattr(_self, 'bytearray', bytearray)

# NotImplemented is the one word of that kind this module cannot carry.
# The reader keeps the name for the answer an operation gives when it
# declines a pair of operands, and refuses to let a module hold anything
# under it, so builtins.NotImplemented is absent here. The value itself
# is reachable: a program writes NotImplemented and gets it.

# What CPython's builtins holds and this one does not, so that a reader
# looking for a missing name learns it is missing rather than broken.
# There is no object behind any of these here: aiter, anext, ascii,
# breakpoint, copyright, credits, exit, help, license, memoryview, open,
# quit, __build_class__, GeneratorExit, StopAsyncIteration, BufferError,
# MemoryError, ReferenceError, SystemError, FloatingPointError,
# IndentationError, TabError, ModuleNotFoundError, the three
# UnicodeError kinds that carry encoding detail, and the OSError kinds
# the operating system raises -- BlockingIOError, BrokenPipeError,
# ChildProcessError, ConnectionError and its four kinds,
# FileExistsError, FileNotFoundError, InterruptedError,
# IsADirectoryError, NotADirectoryError, PermissionError,
# ProcessLookupError and TimeoutError -- along with the old spellings
# EnvironmentError and IOError.
