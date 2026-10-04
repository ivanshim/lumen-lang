# Host primitives used by class bodies need non-private module bindings.
_host_file_exists = __file_exists
_host_file_kind = __file_kind
_host_file_read = __file_read
_host_file_write = __file_write

# The names a program reaches without naming a module at all.
#
# A name a program writes without having bound it is looked for here
# once the kernel has run out of places of its own, which is what the
# language means by naming this module under ext.system.names.module. A
# name the kernel already knows never reaches this far, so rebinding
# builtins.len changes what builtins.len answers and nothing else; a
# name only this module holds, such as open, comes from here alone,
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

__build_class__ = __build_class__

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
UnicodeEncodeError = UnicodeEncodeError
UnicodeDecodeError = UnicodeDecodeError
UnicodeTranslateError = UnicodeTranslateError
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
setattr(__load_module('builtins'), '__debug__', __program_namespace().get('__debug__', True))

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
            self._dirty = True
            if _host_file_exists(name) and _host_file_kind(name) == 2:
                raise IsADirectoryError(21, 'Is a directory', name)
            self._buffer = b'' if self._binary else ''
        elif appending:
            if _host_file_exists(name):
                if _host_file_kind(name) == 2:
                    raise IsADirectoryError(21, 'Is a directory', name)
                brought = _host_file_read(name)
                self._buffer = brought if brought is not False else ''
            else:
                self._buffer = ''
            self._pos = len(self._buffer)
        else:
            kind = _host_file_kind(name)
            if kind == 0:
                raise FileNotFoundError(2, 'No such file or directory', name)
            if kind == 2:
                raise IsADirectoryError(21, 'Is a directory', name)
            brought = _host_file_read(name)
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
            data = bytes(data)
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
            wrote = _host_file_write(self.name, self._buffer)
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


def breakpoint(*args, **kws):
    import sys
    try:
        hook = sys.breakpointhook
    except AttributeError:
        raise RuntimeError('lost sys.breakpointhook')
    return hook(*args, **kws)


def _host_open(file, mode='r', buffering=-1, encoding=None, errors=None, newline=None, closefd=True, opener=None):
    # A null byte inside the name is refused before the name is looked
    # at any further, the way the reference refuses it, whatever the
    # mode.
    if isinstance(file, (bytes, bytearray)) and b'\x00' in bytes(file):
        raise ValueError('embedded null byte')
    if not isinstance(file, str):
        raise TypeError("expected str, bytes or os.PathLike object, not " + type(file).__name__)
    if '\x00' in file:
        raise ValueError('embedded null byte')
    for letter in mode:
        if letter not in 'rwaxb+t':
            raise ValueError("invalid mode: '" + mode + "'")
    return _HostFile(file, mode, encoding, errors)


def open(*args, **kwargs):
    from io import open as io_open
    return io_open(*args, **kwargs)


# One-dimensional views use byte offsets into the original storage. A slice
# keeps those offsets, and a cast groups them without copying the storage.
class memoryview:
    def __buffer__(self, flags, /):
        from _buffer import getbuffer
        return getbuffer(self, flags)

    def __release_buffer__(self, view, /):
        from _buffer import releasebuffer
        return releasebuffer(self, view)

    def __init__(self, object):
        self._init_buffer(object, 284)
    def _init_buffer(self, object, flags):
        from array import array
        self._object = object.obj if isinstance(object, memoryview) else object
        self._owner = None
        self._owner_view = None
        if not isinstance(object, (memoryview, bytes, bytearray, array)) and hasattr(type(object), '__buffer__'):
            self._owner = object
            object = type(object).__buffer__(object, flags)
            if not isinstance(object, memoryview):
                raise TypeError('__buffer__ returned non-memoryview')
            self._owner_view = object
        if isinstance(object, memoryview):
            object._check()
            self._source = object._source
            self._offsets = object._offsets[:]
            self._format = object._format
            self._itemsize = object._itemsize
            self._readonly = object._readonly
        elif isinstance(object, bytes) or isinstance(object, bytearray):
            self._source = object
            self._offsets = list(range(bytes.__len__(object) if isinstance(object, bytes) else bytearray.__len__(object)))
            self._format = 'B'
            self._itemsize = 1
            self._readonly = isinstance(object, bytes)
        elif isinstance(object, array):
            self._source = object
            self._offsets = list(range(0, len(object.data) * object.itemsize, object.itemsize))
            self._format = object.typecode
            self._itemsize = object.itemsize
            self._readonly = False
        else:
            raise TypeError("memoryview: a bytes-like object is required, not '" + type(object).__name__ + "'")
        self._released = False
        self._export = _export(self._source) if isinstance(self._source, bytearray) else None

    def _check(self):
        if self._released:
            raise ValueError('operation forbidden on released memoryview object')

    def _byte(self, offset):
        from array import array
        if isinstance(self._source, array):
            if self._source.typecode in ('B', 'b'):
                return self._source.data[offset] & 255
            return self._source.tobytes()[offset]
        if isinstance(self._source, bytes):
            return bytes.__getitem__(self._source, offset)
        return bytearray.__getitem__(self._source, offset)

    def _put_byte(self, offset, value):
        from array import array
        if isinstance(self._source, array):
            if self._source.typecode in ('B', 'b'):
                self._source.data[offset] = value - 256 if self._source.typecode == 'b' and value >= 128 else value
            else:
                source = self._source
                index = offset // source.itemsize
                start = index * source.itemsize
                data = bytearray(source.tobytes()[start:start + source.itemsize])
                data[offset % source.itemsize] = value
                element = array(source.typecode)
                element.frombytes(data)
                source.data[index] = element.data[0]
        else:
            self._source[offset] = value

    @property
    def obj(self):
        self._check()
        return self._object

    @property
    def format(self):
        self._check()
        return self._format

    @property
    def itemsize(self):
        self._check()
        return self._itemsize

    @property
    def readonly(self):
        self._check()
        return self._readonly

    @property
    def c_contiguous(self):
        self._check()
        if not self._offsets:
            return True
        return all(at == self._offsets[0] + i * self._itemsize for i, at in enumerate(self._offsets))

    @property
    def contiguous(self):
        return self.c_contiguous

    @property
    def f_contiguous(self):
        return self.c_contiguous

    @property
    def nbytes(self):
        return len(self) * self.itemsize

    def __len__(self):
        self._check()
        return len(self._offsets)

    def __getitem__(self, key):
        self._check()
        if isinstance(key, slice):
            child = memoryview(self)
            child._offsets = self._offsets[key]
            return child
        try:
            first = self._offsets[key]
        except IndexError:
            raise IndexError('index out of bounds on dimension 1')
        except TypeError:
            raise TypeError('memoryview: invalid slice key')
        raw = bytes([self._byte(first + i) for i in range(self._itemsize)])
        return int.from_bytes(raw, 'little', signed=self._format in ('b', 'i'))

    def __setitem__(self, key, value):
        self._check()
        if self._readonly:
            raise TypeError('cannot modify read-only memory')
        if isinstance(key, slice):
            places = self._offsets[key]
            if not isinstance(value, (bytes, bytearray, memoryview)):
                raise TypeError('a bytes-like object is required, not ' + type(value).__name__)
            if self._format not in ('B', 'b'):
                raise NotImplementedError('memoryview slice assignment requires a byte format')
            values = list(value)
            if len(places) != len(values):
                raise ValueError('memoryview assignment: lvalue and rvalue have different structures')
            for at, item in zip(places, values):
                self._put_byte(at, item % 256)
        else:
            try:
                first = self._offsets[key]
            except IndexError:
                raise IndexError('index out of bounds on dimension 1')
            except TypeError:
                raise TypeError('memoryview: invalid slice key')
            try:
                raw = value.to_bytes(self._itemsize, 'little', signed=self._format in ('b', 'i'))
            except OverflowError:
                raise ValueError("memoryview: invalid value for format '" + self._format + "'")
            for i in range(self._itemsize):
                self._put_byte(first + i, raw[i])

    def tolist(self):
        self._check()
        return [self[i] for i in range(len(self))]

    def tobytes(self):
        self._check()
        return bytes([self._byte(at + i) for at in self._offsets for i in range(self._itemsize)])

    def __bytes__(self):
        return self.tobytes()

    def __int__(self):
        return int(self.tobytes())

    def __float__(self):
        try:
            return float(self.tobytes())
        except ValueError:
            raise ValueError('could not convert string to float: ' + repr(self))

    def hex(self, sep=None, bytes_per_sep=1):
        if sep is None:
            return self.tobytes().hex()
        return self.tobytes().hex(sep, bytes_per_sep)

    def cast(self, format, shape=None):
        self._check()
        if shape is not None and (not isinstance(shape, (list, tuple)) or len(shape) != 1):
            raise TypeError('memoryview: multi-dimensional casts are not supported')
        if format not in ('B', 'b', 'i', 'I'):
            raise TypeError('memoryview: destination format must be a native single character format')
        if self._offsets:
            start = self._offsets[0]
            for index, offset in enumerate(self._offsets):
                if offset != start + index * self._itemsize:
                    raise TypeError('memoryview: casts are restricted to C-contiguous views')
        else:
            start = 0
        width = 4 if format in ('i', 'I') else 1
        if self.nbytes % width:
            raise TypeError('memoryview: length is not a multiple of itemsize')
        if shape is not None and shape[0] != self.nbytes // width:
            raise TypeError('memoryview: product(shape) * itemsize != buffer size')
        result = memoryview(self)
        result._format = format
        result._itemsize = width
        result._offsets = list(range(start, start + self.nbytes, width))
        return result

    def __del__(self):
        if hasattr(self, '_released'):
            self.release()

    def release(self):
        if self._released:
            return
        self._released = True
        self._export = None
        self._object = None
        if self._owner is not None:
            callback = getattr(type(self._owner), '__release_buffer__', None)
            if callback is not None:
                callback(self._owner, self._owner_view)
            self._owner = None
            self._owner_view = None

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, kind, value, traceback):
        self.release()

    def __eq__(self, other):
        from array import array
        self._check()
        if isinstance(other, array):
            other = memoryview(other)
        if isinstance(other, (memoryview, bytes, bytearray)):
            return self.tolist() == list(other)
        return NotImplemented

    def __hash__(self):
        self._check()
        if not self._readonly:
            raise ValueError('cannot hash writable memoryview object')
        if self._format not in ('B', 'b'):
            raise ValueError("memoryview: hashing is restricted to formats 'B', 'b' or 'c'")
        return hash(self.tobytes())

    def __repr__(self):
        return '<memory at 0x%x>' % id(self)

# What CPython's builtins holds and this one does not, so that a reader
# looking for a missing name learns it is missing rather than broken.
# There is no object behind any of these here: aiter, anext, ascii,
# copyright, credits, exit, help, license,
# quit, GeneratorExit, StopAsyncIteration, BufferError,
# MemoryError, ReferenceError, SystemError, FloatingPointError,
# IndentationError, TabError, and the OSError kinds
# the operating system raises besides FileNotFoundError and
# IsADirectoryError -- BlockingIOError, BrokenPipeError,
# ChildProcessError, ConnectionError and its four kinds,
# FileExistsError, InterruptedError, NotADirectoryError,
# PermissionError, ProcessLookupError and TimeoutError -- along with
# the old spellings EnvironmentError and IOError.


def _buffer_bytes(source):
    with memoryview(source) as view:
        if not view.c_contiguous:
            raise BufferError('memoryview: underlying buffer is not C-contiguous')
        return view.tobytes()


def _buffer_view(source, flags):
    view = object.__new__(memoryview)
    view._init_buffer(source, flags)
    return view
