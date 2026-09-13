# The names a program reaches without naming a module at all.
#
# A name a program writes without having bound it is looked for here
# once the kernel has run out of places of its own, which is what the
# language means by naming this module under ext.system.names.module. A
# name the kernel already knows never reaches this far, so rebinding
# builtins.len changes what builtins.len answers and nothing else, while
# a name only this module holds, such as sentinel, is reached by writing
# it and by nothing else.
#
# Each name below is bound to the builtin of the same name, so what the
# module hands out is the thing itself and not a copy that behaves like
# it. The two classes at the foot of the file are the exception: the
# kernel has no value of either kind, so the module carries them as
# ordinary Python. The names CPython has and this runtime does not are
# simply absent, so hasattr says no for them; they are listed last.

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
# A module's names do not fall back on the reader's word for the root
# class, so object reads as nothing inside one. A class of that name
# stands on the root all the same, and the list of bases it was built on
# hands the real class over, which the name then stands for instead.
class object:
    pass
object = object.__bases__[0]
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
# names a program asks this module for. The loader hands back the module
# it has already begun, so asking six times costs no more than asking
# once and leaves no name of its own standing here afterwards.
setattr(__load_module('builtins'), 'True', True)
setattr(__load_module('builtins'), 'False', False)
setattr(__load_module('builtins'), 'None', None)
setattr(__load_module('builtins'), 'Ellipsis', ...)
setattr(__load_module('builtins'), 'bytes', bytes)
setattr(__load_module('builtins'), 'bytearray', bytearray)

# NotImplemented is the one word of that kind this module cannot carry.
# The reader keeps the name for the answer an operation gives when it
# declines a pair of operands, and refuses to let a module hold anything
# under it, so builtins.NotImplemented is absent here. The value itself
# is reachable: a program writes NotImplemented and gets it.

# A unique thing that stands for itself and prints as its name. The
# kernel has no value of this kind, so the class is written out here: a
# program reaches it by writing sentinel, and each call makes one more.
# What this cannot carry is the class itself being sealed against a
# written attribute, and the module a sentinel says it came from, which
# in CPython is the module that called for it and here is this one.
class sentinel:
    def __init_subclass__(cls, **named):
        raise TypeError("type 'sentinel' is not an acceptable base type")

    def __new__(cls, *given, **named):
        shown = None
        for word in named:
            if word != 'repr':
                raise TypeError("sentinel() takes no keyword argument '" + word + "'")
            shown = named[word]
        if len(given) != 1:
            raise TypeError("sentinel() takes exactly one argument (" + str(len(given)) + " given)")
        if not isinstance(given[0], str):
            raise TypeError("sentinel() argument 'name' must be str, not " + type(given[0]).__name__)
        if shown is not None and not isinstance(shown, str):
            raise TypeError("sentinel() argument 'repr' must be str or None, not " + type(shown).__name__)
        made = super().__new__(cls)
        made.__name__ = given[0]
        made.__module__ = 'builtins'
        made._shown = shown
        return made

    def __init__(self, *given, **named):
        pass

    def __repr__(self):
        if self._shown is None:
            return self.__name__
        return self._shown

    def __bool__(self):
        return True

    def __eq__(self, other):
        return self is other

    def __ne__(self, other):
        return self is not other

    def __hash__(self):
        return id(self)


# A mapping that is read and never written. It keeps an ordinary
# dictionary of its own and hands out that dictionary's views, so what a
# program may do with a frozendict is what it may do with a dictionary
# short of changing it. Nothing stops a program writing into the
# dictionary it holds by reaching for the member it is kept under, so
# this is an immutable mapping by manner rather than by construction.
class frozendict:
    def __new__(cls, *given, **named):
        if len(given) > 1:
            raise TypeError("frozendict expected at most 1 argument, got " + str(len(given)))
        if cls is frozendict and len(given) == 1 and not named and type(given[0]) is frozendict:
            return given[0]
        rows = {}
        if given:
            source = given[0]
            if isinstance(source, frozendict):
                source = source._rows
            if isinstance(source, dict):
                for key in source:
                    rows[key] = source[key]
            else:
                for pair in source:
                    rows[pair[0]] = pair[1]
        for key in named:
            rows[key] = named[key]
        made = super().__new__(cls)
        made._rows = rows
        return made

    # Making one is all the making there is: a second call on a
    # frozendict already made changes nothing about it.
    def __init__(self, *given, **named):
        pass

    def __getitem__(self, key):
        return self._rows[key]

    def __len__(self):
        return len(self._rows)

    def __iter__(self):
        return iter(self._rows)

    def __contains__(self, key):
        return key in self._rows

    def __eq__(self, other):
        if isinstance(other, frozendict):
            return self._rows == other._rows
        if isinstance(other, dict):
            return self._rows == other
        return NotImplemented

    # The rows are gathered in whatever order they were written in, so
    # the hash is made by mixing each pair on its own and folding the
    # results together in a way that does not mind the order.
    def __hash__(self):
        folded = 0
        for key in self._rows:
            folded ^= hash(key) * 1000003 ^ hash(self._rows[key])
        return folded

    def __repr__(self):
        if not self._rows:
            return type(self).__name__ + "()"
        return type(self).__name__ + "(" + repr(self._rows) + ")"

    def __or__(self, other):
        if isinstance(other, frozendict):
            other = other._rows
        elif not isinstance(other, dict):
            return NotImplemented
        if not other and type(self) is frozendict:
            return self
        return frozendict(self._rows | other)

    def __ror__(self, other):
        if not isinstance(other, dict):
            return NotImplemented
        if not self._rows and type(self) is frozendict:
            return self
        return frozendict(other | self._rows)

    def keys(self):
        return self._rows.keys()

    def values(self):
        return self._rows.values()

    def items(self):
        return self._rows.items()

    def get(self, key, otherwise=None):
        return self._rows.get(key, otherwise)

    # One already frozen needs no copy; anything standing on it does.
    def copy(self):
        if type(self) is frozendict:
            return self
        return frozendict(self._rows)

    @classmethod
    def fromkeys(cls, keys, value=None):
        rows = {}
        for key in keys:
            rows[key] = value
        return cls(rows)


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
