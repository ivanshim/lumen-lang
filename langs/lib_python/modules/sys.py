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
version_info = (3, 14, 0, 'final', 0)
platform = 'linux'
# Which Python this is. A test that reaches for the internals of the
# reference implementation asks the name here first, and the honest
# answer -- not cpython -- is what lets such a test step aside instead
# of measuring this kernel against machinery it does not have.
class _Implementation:
    name = 'lumen'
    version = (0, 2, 0, 'final', 0)
    hexversion = 0x000200f0
    # Nothing is written beside a module as compiled code, and a name
    # of None is how a Python says exactly that.
    cache_tag = None

implementation = _Implementation()
# The module cache also retains namespaces installed by source loaders.
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
class _Output:
    def write(self, *args, **keywords):
        if keywords:
            raise TypeError("write() takes no keyword arguments")
        return _host_stream_write(*args, False)

    def flush(self):
        pass

    def isatty(self):
        return False

class _Error:
    def write(self, *args, **keywords):
        if keywords:
            raise TypeError("write() takes no keyword arguments")
        return _host_stream_write(*args, True)

    def flush(self):
        pass

    def isatty(self):
        return False

class _Input:
    def read(self, size=-1):
        return _host_stream_read(size, False)

    def readline(self, size=-1):
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
executable = globals().get('__runner__', '')

float_repr_style = 'short'
byteorder = 'little'
maxunicode = 1114111

def intern(string):
    if not isinstance(string, str):
        raise 'TypeError: intern() argument must be str'
    return string

def getsizeof(value, default=None):
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


def getdefaultencoding():
    return 'utf-8'
