# The names a program reaches without naming a module at all.
#
# A name a program writes without having bound it is looked for here
# once the kernel has run out of places of its own, which is what the
# language means by naming this module under ext.system.names.module. A
# name the kernel already knows never reaches this far, so rebinding
# builtins.len changes what builtins.len answers and nothing else; a
# name only this module holds, such as sentinel, comes from here alone,
# and writing a new value onto the module under it changes what the bare
# name answers with.
#
# Each name below is bound to the builtin of the same name, so what the
# module hands out is the thing itself and not a copy that behaves like
# it. The two classes near the foot of the file are the exception: the
# kernel has no value of either kind, so the module carries them as
# ordinary Python. The names CPython has and this runtime does not are
# simply absent, so hasattr says no for them, and they are listed at the
# very end.

ArithmeticError = ArithmeticError
AssertionError = AssertionError
AttributeError = AttributeError
BaseException = BaseException
BaseExceptionGroup = BaseExceptionGroup
EOFError = EOFError
Exception = Exception
ExceptionGroup = ExceptionGroup
ImportError = ImportError
ModuleNotFoundError = ModuleNotFoundError
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
# rather than into a module's, so it is read from there. Text run with a
# dictionary of its own for the outermost names has no such seed, and a
# module first asked for while that text runs finds nothing there, so
# the answer falls back on what the reader would have seeded: there is
# no switch in this runtime that turns the checks off, so it is always
# true.
__debug__ = globals().get('__debug__', True)

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


FileNotFoundError = FileNotFoundError
IsADirectoryError = IsADirectoryError

# A file read from or written to the host's own disk. What backs it is
# whichever whole-file primitive the kernel carries -- a read brings
# the whole file in once, at open time; a write is kept here and goes
# out whole, at close (or at an explicit flush) -- so nothing here
# streams, but everything the two reference tests that need open() ask
# of a file, this file answers.
class _HostFile:
    def __init__(self, name, mode, encoding=None, errors=None):
        self.name = name
        self.mode = mode
        self._binary = 'b' in mode
        self.encoding = None if self._binary else (encoding if encoding is not None else 'utf-8')
        self.errors = errors if errors is not None else 'strict'
        self.closed = False
        self._pos = 0
        self._dirty = False
        # Line-at-a-time reading is what both the reference tests and
        # the probe lean on hardest, so the whole of what is left to
        # read is split into lines once, the first time a line is
        # asked for, rather than this file being rescanned from the
        # front for every `\n` -- letting a many-line file be walked
        # in time proportional to its own length rather than its
        # square. `read` and `seek` empty this cache, since either may
        # move `_pos` somewhere the cache does not account for; the
        # next line asked for after either then rebuilds it.
        self._lines = None
        self._lines_at = 0
        self._lines_pos = 0
        reading = 'r' in mode or '+' in mode and 'w' not in mode and 'a' not in mode
        writing = 'w' in mode
        appending = 'a' in mode
        if not (reading or writing or appending):
            reading = True
        if writing:
            if __file_exists(name) and __file_kind(name) == 2:
                raise IsADirectoryError(21, 'Is a directory', name)
            self._buffer = ''
        elif appending:
            if __file_exists(name):
                if __file_kind(name) == 2:
                    raise IsADirectoryError(21, 'Is a directory', name)
                brought = __file_read(name)
                self._buffer = brought if brought is not False else ''
            else:
                self._buffer = ''
            self._pos = len(self._buffer)
        else:
            kind = __file_kind(name)
            if kind == 0:
                raise FileNotFoundError(2, 'No such file or directory', name)
            if kind == 2:
                raise IsADirectoryError(21, 'Is a directory', name)
            brought = __file_read(name)
            if brought is False:
                raise OSError(5, 'Input/output error', name)
            self._buffer = brought

    def _open(self):
        if self.closed:
            raise ValueError('I/O operation on closed file.')

    def readable(self):
        return 'r' in self.mode or '+' in self.mode

    def writable(self):
        return 'w' in self.mode or 'a' in self.mode or '+' in self.mode

    def _carried(self, text):
        return text.encode('utf-8') if self._binary else text

    def read(self, size=-1):
        self._open()
        if size is None or size < 0:
            size = len(self._buffer) - self._pos
        value = self._buffer[self._pos:self._pos + size]
        self._pos += len(value)
        self._lines = None
        return self._carried(value)

    def _ensure_lines(self):
        if self._lines is None or self._lines_pos != self._pos:
            self._lines = self._buffer[self._pos:].splitlines(keepends=True)
            self._lines_at = 0
            self._lines_pos = self._pos

    def readline(self, size=-1):
        self._open()
        if size is not None and size >= 0:
            start = self._pos
            limit = min(len(self._buffer), start + size)
            found = self._buffer.find('\n', start, limit)
            stop = limit if found < 0 else found + 1
            self._pos = stop
            self._lines = None
            return self._carried(self._buffer[start:stop])
        self._ensure_lines()
        if self._lines_at >= len(self._lines):
            return self._carried('')
        line = self._lines[self._lines_at]
        self._lines_at += 1
        self._pos += len(line)
        self._lines_pos = self._pos
        return self._carried(line)

    def readlines(self, hint=-1):
        self._open()
        if hint is None or hint < 0:
            self._ensure_lines()
            remaining = self._lines[self._lines_at:]
            self._lines_at = len(self._lines)
            self._pos += sum(len(line) for line in remaining)
            self._lines_pos = self._pos
            if self._binary:
                return [self._carried(line) for line in remaining]
            return remaining
        lines = []
        total = 0
        while True:
            line = self.readline()
            if line == self._carried(''):
                return lines
            lines.append(line)
            total += len(line)
            if total >= hint:
                return lines

    def __iter__(self):
        self._open()
        return self

    def __next__(self):
        line = self.readline()
        if line == self._carried(''):
            raise StopIteration
        return line

    def write(self, data):
        self._open()
        if not self.writable():
            raise ValueError('File not open for writing')
        if self._binary:
            if not isinstance(data, bytes) and not isinstance(data, bytearray):
                raise TypeError('a bytes-like object is required, not ' + type(data).__name__)
            data = bytes(data).decode('utf-8')
        elif not isinstance(data, str):
            raise TypeError('write() argument must be str, not ' + type(data).__name__)
        self._buffer = self._buffer[:self._pos] + data + self._buffer[self._pos + len(data):]
        self._pos += len(data)
        self._dirty = True
        self._lines = None
        return len(data)

    def writelines(self, lines):
        for line in lines:
            self.write(line)

    def flush(self):
        self._open()
        if self._dirty:
            wrote = __file_write(self.name, self._buffer)
            if wrote is False:
                raise FileNotFoundError(2, 'No such file or directory', self.name)
            self._dirty = False

    def close(self):
        if self.closed:
            return
        if self.writable():
            self.flush()
        self.closed = True

    def __enter__(self):
        self._open()
        return self

    def __exit__(self, kind, value, traceback):
        self.close()
        return False

    def tell(self):
        self._open()
        return self._pos

    def seekable(self):
        return True

    def seek(self, offset, whence=0):
        self._open()
        if whence == 1:
            offset += self._pos
        elif whence == 2:
            offset += len(self._buffer)
        self._pos = offset
        return self._pos

    def __repr__(self):
        return "<_io.TextIOWrapper name='" + self.name + "' mode='" + self.mode + "'>"


def open(file, mode='r', buffering=-1, encoding=None, errors=None, newline=None, closefd=True, opener=None):
    if not isinstance(file, str):
        raise TypeError("expected str, bytes or os.PathLike object, not " + type(file).__name__)
    for letter in mode:
        if letter not in 'rwaxb+t':
            raise ValueError("invalid mode: '" + mode + "'")
    return _HostFile(file, mode, encoding, errors)


# What CPython's builtins holds and this one does not, so that a reader
# looking for a missing name learns it is missing rather than broken.
# There is no object behind any of these here: aiter, anext, ascii,
# breakpoint, copyright, credits, exit, help, license, memoryview,
# quit, __build_class__, GeneratorExit, StopAsyncIteration, BufferError,
# MemoryError, ReferenceError, SystemError, FloatingPointError,
# IndentationError, TabError, the three
# UnicodeError kinds that carry encoding detail, and the OSError kinds
# the operating system raises besides FileNotFoundError and
# IsADirectoryError -- BlockingIOError, BrokenPipeError,
# ChildProcessError, ConnectionError and its four kinds,
# FileExistsError, InterruptedError, NotADirectoryError,
# PermissionError, ProcessLookupError and TimeoutError -- along with
# the old spellings EnvironmentError and IOError.
