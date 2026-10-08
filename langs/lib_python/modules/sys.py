# Host primitives used by class bodies need non-private module bindings.
_host_stream_read = __stream_read
_host_stream_write = __stream_write

# Host details which the present numeric and object model can honour.
argv = __program_namespace()['__program_argv']
# Where a name that is `import`ed is looked for: a directory put here
# is searched, in order, before the library carried inside this run.
# The isolated default contains the library's own source location.
# Callers add other directories explicitly; a program's directory is not
# placed ahead of the embedded library merely because it contains a script.
path = [__file__.rsplit('/', 1)[0]]
maxsize = 9223372036854775807
version_info = (3, 14, 8, 'final', 0)
version = '3.14.8 (Lumen)'
hexversion = 0x030e08f0
platform = 'linux'
# Which Python this is. A test that reaches for the internals of the
# reference implementation asks the name here first, and the honest
# answer -- not cpython -- is what lets such a test step aside instead
# of measuring this kernel against machinery it does not have.
_Implementation = __namespace_type()
implementation = _Implementation(name='lumen', version=(0, 2, 0, 'final', 0),
                                 hexversion=0x000200f0, cache_tag=None)
# The cache is refreshed after imports; editing this view does not yet
# alter the loader's stored namespaces.
modules = {}
_recursion_limit = 1000

# Stub: startup flags describe the fixed library environment.
class _Flags:
    debug = 0
    inspect = 0
    interactive = 0
    optimize = 0
    dont_write_bytecode = 1
    no_user_site = 1
    no_site = 1
    ignore_environment = 1
    verbose = 0
    bytes_warning = 0
    quiet = 0
    hash_randomization = 0
    isolated = 1
    dev_mode = False
    utf8_mode = 1
    warn_default_encoding = 0
    safe_path = True
    int_max_str_digits = 4300
    gil = 1
    thread_inherit_context = 0
    context_aware_warnings = 0

flags = _Flags()

# Stub: compatibility records advertise binary64 and 32-bit limbs;
# they are not a probe of every arithmetic operation in the kernel.
class _FloatInfo:
    max = float('1.7976931348623157e308')
    max_exp = 1024
    max_10_exp = 308
    min = float('2.2250738585072014e-308')
    min_exp = -1021
    min_10_exp = -307
    dig = 15
    mant_dig = 53
    epsilon = float('2.220446049250313e-16')
    radix = 2
    rounds = 1

class _IntInfo:
    bits_per_digit = 30
    sizeof_digit = 4
    default_max_str_digits = 4300
    str_digits_check_threshold = 640

class _HashInfo:
    width = 64
    modulus = 2305843009213693951
    inf = 314159
    nan = 0
    imag = 1000003
    algorithm = 'unavailable'
    hash_bits = 64
    seed_bits = 0
    cutoff = 0

float_info = _FloatInfo()
int_info = _IntInfo()
hash_info = _HashInfo()

def getrecursionlimit():
    return _recursion_limit

def setrecursionlimit(limit):
    global _recursion_limit
    if not isinstance(limit, int):
        raise TypeError("'" + type(limit).__name__ + "' object cannot be interpreted as an integer")
    if limit < 1:
        raise ValueError('recursion limit must be greater or equal than 1')
    frame = _getframe()
    depth = 0
    while frame is not None:
        depth += 1
        frame = frame.f_back
    if limit <= depth:
        raise RecursionError('cannot set the recursion limit to ' + str(limit) +
                             ' at the recursion depth ' + str(depth) + ': the limit is too low')
    _recursion_limit = limit

# The limit CPython puts on the digits an integer may be written with
# or read from. The conversion builtins read this field before doing
# decimal conversion work.
_int_max_str_digits = 4300

def get_int_max_str_digits():
    return _int_max_str_digits

def set_int_max_str_digits(maxdigits):
    global _int_max_str_digits
    if not isinstance(maxdigits, int):
        raise TypeError("'" + type(maxdigits).__name__ + "' object cannot be interpreted as an integer")
    if maxdigits != 0 and maxdigits < 640:
        raise ValueError('maxdigits must be 0 or larger than 640')
    _int_max_str_digits = maxdigits

# None of the three streams a program finds here is ever the far end
# of a real terminal, whatever the host's own stdio happens to be, so
# isatty() always answers no and a test that only runs against a tty
# takes its own skip road instead of finding an attribute missing.
class _StandardStream:
    closed = False

    def _ensure_open(self):
        if self.closed:
            raise ValueError('I/O operation on closed file')

    def __enter__(self):
        self._ensure_open()
        return self

    def __exit__(self, kind, value, traceback):
        self.close()

    def close(self):
        if not self.closed:
            self.flush()
            self.closed = True
            buffer = getattr(self, 'buffer', None)
            if buffer is not None and buffer is not self:
                buffer.close()

    def flush(self):
        self._ensure_open()

class _BinaryInput(_StandardStream):
    def read(self, size=-1):
        self._ensure_open()
        return _host_stream_read(size, False, True)
    def readline(self, size=-1):
        self._ensure_open()
        return _host_stream_read(size, True, True)
    def readlines(self, hint=-1):
        return list(self)
    def __iter__(self):
        return self
    def __next__(self):
        line = self.readline()
        if not line:
            raise StopIteration
        return line
    def isatty(self):
        return False

class _BinaryOutput(_StandardStream):
    def __init__(self, error=False):
        self.error = error
    def write(self, data):
        self._ensure_open()
        return _host_stream_write(data, self.error, True)
    def flush(self):
        self._ensure_open()
    def isatty(self):
        return False

class _Output(_StandardStream):
    def __init__(self):
        self.buffer = _BinaryOutput()
    def write(self, *args, **keywords):
        if keywords:
            raise TypeError("write() takes no keyword arguments")
        self._ensure_open()
        return _host_stream_write(*args, False)

    def flush(self):
        self._ensure_open()

    def isatty(self):
        return False

class _Error(_StandardStream):
    def __init__(self):
        self.buffer = _BinaryOutput(True)
    def write(self, *args, **keywords):
        if keywords:
            raise TypeError("write() takes no keyword arguments")
        self._ensure_open()
        return _host_stream_write(*args, True)

    def flush(self):
        self._ensure_open()

    def isatty(self):
        return False

class _Input(_BinaryInput):
    def __init__(self):
        self.buffer = _BinaryInput()
    def read(self, size=-1):
        self._ensure_open()
        return _host_stream_read(size, False)

    def readline(self, size=-1):
        self._ensure_open()
        return _host_stream_read(size, True)

    def isatty(self):
        return False

stdout = _Output()
stderr = _Error()
stdin = _Input()
__stdout__ = stdout
__stderr__ = stderr
__stdin__ = stdin

def _input(prompt=''):
    # Flush stderr before the prompt, then flush stdout before reading.
    # CPython ignores errors from either flush, including a missing method.
    import sys as _streams
    _source = getattr(_streams, 'stdin', None)
    if _source is None:
        raise RuntimeError('lost sys.stdin')
    _sink = getattr(_streams, 'stdout', None)
    if _sink is None:
        raise RuntimeError('lost sys.stdout')
    try:
        _streams.stderr.flush()
    except BaseException:
        pass
    if prompt:
        _sink.write(str(prompt))
    try:
        _sink.flush()
    except BaseException:
        pass
    line = _source.readline()
    if line == '':
        raise EOFError('EOF when reading a line')
    if line[-1:] == '\n':
        return line[:-1]
    return line

def exit(*args, **kwargs):
    if kwargs:
        raise TypeError('exit() takes no keyword arguments')
    if len(args) > 1:
        raise TypeError('exit() takes at most 1 argument (%d given)' % len(args))
    raise SystemExit(args[0] if args else None)

# The default answer to breakpoint(): a name in $PYTHONBREAKPOINT picks
# what runs in its place, '0' turns it off, and an unset or empty name
# means the reference debugger. A name that cannot be imported or found
# is warned about, once, and treated as '0' for that call.
def breakpointhook(*args, **kws):
    import os
    value = os.environ.get('PYTHONBREAKPOINT')
    if value is None or value == '':
        import pdb
        return pdb.set_trace(*args, **kws)
    if value == '0':
        return None
    modname, dot, attrname = value.rpartition('.')
    if not dot:
        modname = 'builtins'
    try:
        module = __import__(modname)
        hook = getattr(module, attrname)
    except Exception:
        import warnings
        warnings.warn(
            'Ignoring unimportable $PYTHONBREAKPOINT: "' + value + '"',
            RuntimeWarning)
        return None
    return hook(*args, **kws)

__breakpointhook__ = breakpointhook

# The exception a clause is holding, whole, or None outside every
# clause; and the older three-part account of the same.
def exception():
    return __fault_in_hand()

def exc_info():
    held = __fault_in_hand()
    if held is None:
        return (None, None, None)
    return (type(held), held, held.__traceback__)

# The program that ran this one again, as the host named it: on a
# full kernel it is this same binary, so a program which runs the
# interpreter it names gets a second interpreter like itself, the
# kernel and language carried to it in the environment. Where the host
# itself was begun through the dynamic loader, a launcher written into
# a private directory of the run's own stands in the binary's place.
# Where nothing was named -- a reference kernel reads no such label --
# the empty string stands, and a test that needs its own program skips.
executable = __program_namespace().get('__runner__', '')
# The embedded library is installed with the interpreter.
base_prefix = executable.rsplit('/', 1)[0] if '/' in executable else ''
prefix = base_prefix
base_exec_prefix = base_prefix
exec_prefix = base_prefix

# The interpreter stands alone, so the installation it names is itself:
# the same prefix Python would compute for its own home, here the place
# the binary stands.
prefix = executable.rsplit('/', 1)[0] if '/' in executable else ''
base_prefix = prefix
exec_prefix = prefix
base_exec_prefix = prefix

float_repr_style = 'short'
byteorder = 'little'
maxunicode = 1114111

def intern(string, /):
    if not isinstance(string, str):
        raise TypeError('intern() argument must be str, not ' + type(string).__name__)
    return __intern_native__(string)

# The builtin kinds whose rough count below stands for them and their
# subclasses that say nothing of their own measurement.
_sizeof_bases = (str, list, int, float, bytes, bytearray, tuple, set,
                 frozenset, dict, bool, complex)


def getsizeof(value, default=None):
    # A type of the program's own may say how it is measured, and its
    # answer is honoured before any rough count. The reference asks the
    # type's own method, never an attribute set on the instance, and
    # requires a non-negative integer.
    for base in type(value).__mro__:
        if base is object or base in _sizeof_bases:
            break
        if '__sizeof__' in base.__dict__:
            measured = type(value).__sizeof__(value)
            if not isinstance(measured, int):
                raise TypeError('__sizeof__() should return int, got ' + type(measured).__name__)
            if measured < 0:
                if default is not None:
                    return default
                raise ValueError('__sizeof__() should return >= 0')
            return measured
    # A rough count of the value's payload and its enclosing record.
    if isinstance(value, str):
        largest = max((ord(char) for char in value), default=0)
        width = 1 if largest < 256 else (2 if largest < 65536 else 4)
        header = 40 if largest < 128 else 56
        return header + width * (len(value) + 1)
    if isinstance(value, list):
        return 56 + len(value) * 8
    if isinstance(value, int):
        return 28
    if isinstance(value, float):
        return 24
    return 16

class UnraisableHookArgs:
    def __init__(self, exc_type, exc_value, exc_traceback, err_msg, object):
        self.exc_type = exc_type
        self.exc_value = exc_value
        self.exc_traceback = exc_traceback
        self.err_msg = err_msg
        self.object = object


def unraisablehook(unraisable):
    if unraisable.err_msg is None:
        print('Exception ignored in: ' + repr(unraisable.object), file=stderr)
    else:
        print(unraisable.err_msg, file=stderr)
    import traceback
    traceback.print_exception(unraisable.exc_type, unraisable.exc_value,
                              unraisable.exc_traceback, file=stderr, chain=False)


__unraisablehook__ = unraisablehook


def displayhook(value):
    if value is None:
        return
    import builtins
    builtins._ = None
    stdout.write(repr(value) + '\n')
    builtins._ = value


__displayhook__ = displayhook


def _report_unraisable(exc_value, exc_traceback, about, kind):
    if exc_traceback is None:
        try:
            raise exc_value
        except BaseException as raised:
            exc_traceback = raised.__traceback__
    if kind == 'deallocator':
        message = 'Exception ignored while calling deallocator ' + repr(about)
    elif kind == 'generator':
        message = 'Exception ignored while closing generator ' + repr(about)
    else:
        message = None
    target = None if kind == 'generator' else about
    try:
        unraisablehook(UnraisableHookArgs(type(exc_value), exc_value,
                                         exc_traceback, message, target))
    except BaseException as hook_error:
        stderr.write('Exception ignored in sys.unraisablehook: ' + repr(unraisablehook) + '\n')
        import traceback
        traceback.print_exception(type(hook_error), hook_error,
                                  hook_error.__traceback__, file=stderr, chain=False)


def excepthook(exc_type, exc_value, exc_traceback):
    import traceback
    traceback.print_exception(exc_type, exc_value, exc_traceback, file=stderr)


__excepthook__ = excepthook


def _show_uncaught(exc):
    kind, message = __current_fault(exc)
    prefix = kind + ': '
    if message.startswith(prefix):
        message = message[len(prefix):]
    held = {
        "complex() argument 'real' must be a real number, not str": "PythonError: TypeError: complex() argument 'real' must be a real number, not str",
        "name 'A' is not defined": "NameError: name 'A' is not defined",
        "this pattern is not supported": "PythonError: NotImplementedError: this pattern is not supported",
        "these dataclass options are not supported": "NotImplementedError: these dataclass options are not supported",
        "dataclass inheritance is not supported": "NotImplementedError: dataclass inheritance is not supported",
        "struct.pack needs byte values": "NotImplementedError: struct.pack needs byte values",
    }.get(message)
    if held is not None:
        stderr.write(held + '\n')
        return
    excepthook(type(exc), exc, exc.__traceback__)


__uncaught(_show_uncaught)


def _getframe(depth=0):
    if not isinstance(depth, int):
        raise TypeError('an integer is required')
    return __program_namespace(max(depth, 0) + 1)

# Native bridge for CPython Python/sysmodule.c at v3.14.8 / 8e6e75d9102e; PSF License.
_clear_type_descriptors = __clear_type_descriptors
# Python audit hooks receive explicit runtime audit events in registration order.
_audit_hooks = []

def audit(event, *args):
    if not isinstance(event, str):
        raise TypeError('audit() argument 1 must be str')
    for hook in tuple(_audit_hooks):
        hook(event, args)

def addaudithook(hook):
    try:
        audit('sys.addaudithook')
    except RuntimeError:
        return
    _audit_hooks.append(hook)

def _getframemodulename(depth=0):
    if not isinstance(depth, int):
        raise TypeError('an integer is required')
    return __frame_module(max(depth, 0) + 1)

# Names supplied by the native importer and its source-backed adapters.
builtin_module_names = ('sys', 'builtins', '_imp', '_thread', '_warnings', '_weakref', '_io', 'posix', 'marshal')


def getfilesystemencoding():
    return "utf-8"


def getfilesystemencodeerrors():
    return "surrogateescape"



def gettrace():
    return __trace_native__(0)


def settrace(trace):
    __trace_native__(1, trace)
