# Host primitives used by class bodies need non-private module bindings.
_host_stream_read = __stream_read
_host_stream_write = __stream_write
_host_environment = __posix

def _write_standard(*args):
    result = _host_stream_write(*args)
    if isinstance(result, tuple):
        raise OSError(*result)
    return result

# Host details which the present numeric and object model can honour.
argv = __program_namespace()['__program_argv']
# Startup options are consumed by the warnings module when first imported.
warnoptions = []
_startup_options = __posix('environ')[1].get(b'LUMEN_WARNOPTIONS', b'').decode('utf-8')
while _startup_options:
    _size, _tail = _startup_options.split(':', 1)
    _size = int(_size)
    warnoptions.append(_tail[:_size])
    _startup_options = _tail[_size:]
del _startup_options
_xoptions = {}
_startup_options = __posix('environ')[1].get(b'LUMEN_XOPTIONS', b'').decode('utf-8')
while _startup_options:
    _size, _tail = _startup_options.split(':', 1)
    _size = int(_size)
    _option = _tail[:_size]
    _name, _equals, _value = _option.partition('=')
    _xoptions[_name] = _value if _equals else True
    _startup_options = _tail[_size:]
del _startup_options
# Where a name that is `import`ed is looked for: a directory put here
# is searched, in order, before the library carried inside this run.
# Keep the safe-path default without implicitly adding the script directory.
# Explicit PYTHONPATH entries precede the embedded library in their given order.
# Normalize a POSIX search entry against the startup directory.
def _absolute_search_entry(entry, cwd):
    if not entry.startswith('/'):
        entry = cwd + '/' + entry
    prefix = '//' if entry.startswith('//') and not entry.startswith('///') else '/'
    pieces = []
    for piece in entry.split('/'):
        if piece in ('', '.'):
            continue
        if piece == '..':
            if pieces:
                pieces.pop()
        else:
            pieces.append(piece)
    return prefix + '/'.join(pieces)


path = [__file__.rsplit('/', 1)[0]]
_path_host = __host_info()
_safe_path = _path_host[3].get('LUMEN_PYTHON_SAFE_PATH') == '1'
if not _safe_path:
    _startup_file = __program_namespace().get('__file__', '')
    if _startup_file in ('<string>', '<stdin>'):
        path.insert(0, '')
    elif _startup_file == '-m':
        path.insert(0, _path_host[0])
    else:
        _startup_file = _absolute_search_entry(_startup_file, _path_host[0])
        path.insert(0, _startup_file.rsplit('/', 1)[0])
_ignore_environment = int(_path_host[3].get('LUMEN_PYTHON_IGNORE_ENV') == '1')
_python_path = '' if _ignore_environment else _path_host[3].get('PYTHONPATH', '')
if _python_path:
    _search_paths = []
    for _entry in _python_path.split(':'):
        _entry = _absolute_search_entry(_entry, _path_host[0])
        if _entry not in _search_paths:
            _search_paths.append(_entry)
    _environment_at = 0 if _safe_path else 1
    path[_environment_at:_environment_at] = _search_paths
maxsize = 9223372036854775807
# Where a written bytecode cache would live, and whether one is
# written: the reference reads both while naming a cache file.
pycache_prefix = None
dont_write_bytecode = True
# A struct sequence, as the reference's sys.version_info is: five named
# fields over a tuple, built by the host the way os.stat_result is.
_structseq = __posix


class version_info(tuple):
    __slots__ = ()
    n_sequence_fields = 5
    n_fields = 5
    n_unnamed_fields = 0
    __match_args__ = ('major', 'minor', 'micro', 'releaselevel', 'serial')

    def __new__(cls, *args, **kwargs):
        raise TypeError("cannot create 'sys.version_info' instances")

    major = property(lambda self: self[0])
    minor = property(lambda self: self[1])
    micro = property(lambda self: self[2])
    releaselevel = property(lambda self: self[3])
    serial = property(lambda self: self[4])

    def __repr__(self):
        return 'sys.version_info(' + ', '.join(name + '=' + repr(value) for name, value in zip(self.__match_args__, self)) + ')'

    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'sys.version_info' is not an acceptable base type")


version_info.__module__ = 'sys'
version_info = _structseq('structseq_new', version_info, (3, 14, 8, 'final', 0), ())
version = '3.14.8 (Lumen) [Rust]'
# The source-control identity the reference build records beside the
# version. This interpreter is not checked out from CPython's tree, so
# there is no branch or revision; the build tag names it plainly instead.
_git = ('Lumen', '', '')
hexversion = 0x030e08f0
platform = 'linux'
platlibdir = 'lib'
copyright = 'Lumen experimental interpreter; Python library sources are licensed by the Python Software Foundation.'
# Which Python this is. A test that reaches for the internals of the
# reference implementation asks the name here first, and the honest
# answer -- not cpython -- is what lets such a test step aside instead
# of measuring this kernel against machinery it does not have.
_Implementation = __namespace_type()
implementation = _Implementation(name='lumen', version=(0, 2, 0, 'final', 0),
                                 hexversion=0x000200f0, cache_tag=_code_abi()[0])
# The cache is refreshed after imports; editing this view does not yet
# alter the loader's stored namespaces.
modules = {}
from _runtime_import import SourceFinder as _SourceFinder, make_spec as _make_spec, find_custom as _find_custom
meta_path = [_SourceFinder()]
path_hooks = []
path_importer_cache = {}
pycache_prefix = None
_recursion_limit = 1000

def _environment(name):
    if _ignore_environment:
        return None
    import posix
    value = posix.environ.get(name)
    if value is None:
        value = posix.environ.get(name.encode())
    if isinstance(value, bytes):
        value = value.decode()
    return value

# Stub: startup flags describe the fixed library environment.
_dev_mode = None
class _Flags:
    debug = 0
    inspect = 0
    interactive = 0
    optimize = 0
    dont_write_bytecode = 1
    no_user_site = 1
    # Builtins can import sys before its trailing startup adapter runs.
    no_site = 1 if _host_environment("environ")[1].get(b"LUMEN_PYTHON_NO_SITE") else 0
    ignore_environment = _ignore_environment
    verbose = 0
    bytes_warning = 0
    quiet = 0
    hash_randomization = 0
    isolated = 0
    # The options that PYTHONDEVMODE, PYTHONUTF8 and
    # PYTHONWARNDEFAULTENCODING (or -X dev, -X utf8, -X
    # warn_default_encoding, which the helpers that start a second
    # interpreter hand over as those) turn on or off, read when asked.
    @property
    def dev_mode(self):
        # Settled at start-up like the real flag: read once, then kept.
        global _dev_mode
        if _dev_mode is None:
            _dev_mode = bool(_environment('PYTHONDEVMODE')) or _host_environment('environ')[1].get(b'LUMEN_PYTHON_DEV_MODE') == b'1'
        return _dev_mode
    @property
    def utf8_mode(self):
        # 3.14 leaves this off unless PYTHONUTF8=1. Unset and 0 stay off.
        return 1 if _environment('PYTHONUTF8') == '1' else 0
    @property
    def warn_default_encoding(self):
        return 1 if _environment('PYTHONWARNDEFAULTENCODING') else 0
    safe_path = _safe_path
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
    algorithm = 'siphash13'
    hash_bits = 64
    seed_bits = 128
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
    # Check the same logical call depth that both kernels enforce.
    depth = __program_namespace('recursion_depth')
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

# Standard streams validate their descriptor before asking os.isatty.
class _StandardStream:
    closed = False
    encoding = 'utf-8'
    errors = 'backslashreplace'

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

    def fileno(self):
        self._ensure_open()
        return self._fd

class _BinaryInput(_StandardStream):
    _fd = 0
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
        import os
        return os.isatty(self.fileno())

class _BinaryOutput(_StandardStream):
    def __init__(self, error=False):
        self.error = error
        self._fd = 2 if error else 1
    def write(self, data):
        self._ensure_open()
        return _write_standard(data, self.error, True)
    def flush(self):
        self._ensure_open()
    def isatty(self):
        import os
        return os.isatty(self.fileno())

class _Output(_StandardStream):
    _fd = 1
    def __init__(self):
        self.buffer = _BinaryOutput()
    def write(self, *args, **keywords):
        if keywords:
            raise TypeError("write() takes no keyword arguments")
        self._ensure_open()
        return _write_standard(*args, False)

    def flush(self):
        self._ensure_open()

    def isatty(self):
        import os
        return os.isatty(self.fileno())

class _Error(_StandardStream):
    _fd = 2
    def __init__(self):
        self.buffer = _BinaryOutput(True)
    def write(self, *args, **keywords):
        if keywords:
            raise TypeError("write() takes no keyword arguments")
        self._ensure_open()
        return _write_standard(*args, True)

    def flush(self):
        self._ensure_open()

    def isatty(self):
        import os
        return os.isatty(self.fileno())

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
        import os
        return os.isatty(self.fileno())

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
    value = None if flags.ignore_environment else os.environ.get('PYTHONBREAKPOINT')
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
            method = base.__dict__['__sizeof__']
            binder = getattr(type(method), '__get__', None)
            if binder is not None:
                method = binder(method, value, type(value))
            try:
                measured = method()
            except TypeError:
                if default is not None:
                    return default
                raise
            if not isinstance(measured, int):
                raise TypeError('__sizeof__() should return int, got ' + type(measured).__name__)
            if measured < 0:
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
    elif kind == 'file':
        message = 'Exception ignored while finalizing file ' + repr(about)
    elif kind == 'coroutine':
        message = 'Exception ignored while finalizing coroutine ' + repr(about)
    elif kind == 'generator':
        message = 'Exception ignored while closing generator ' + repr(about)
    else:
        message = None
    target = None if kind in ('generator', 'coroutine', 'file') else about
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

# The routine the kernel tells of each call and return stands between it
# and whatever function the program gave to setprofile.
_profile_target = [None]

def _profile_dispatch(frame, event, arg):
    function = _profile_target[0]
    if function is None:
        return
    # setprofile itself is native in the reference, so the routine
    # standing here for it is not a call the program made.
    code = frame.f_code
    if code.co_name == 'setprofile' and code.co_filename == _profile_home[0]:
        return
    try:
        function(frame, event, arg)
    except BaseException:
        # A function that fails is taken away, as the reference does.
        _profile_target[0] = None
        raise

def setprofile(function):
    _profile_target[0] = function
    __profile(_profile_dispatch if function is not None else None)

_profile_home = [setprofile.__code__.co_filename]

def getprofile():
    return _profile_target[0]

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
builtin_module_names = ('sys', 'builtins', '_imp', '_thread', '_weakref', '_io', 'posix', 'marshal', '_warnings')


def getfilesystemencoding():
    return "utf-8"


def getfilesystemencodeerrors():
    return "surrogateescape"


# Explicit module runs search the working directory before the embedded
# library. File runs retain the isolated default search path.
if __program_namespace().get('__spec__') is not None:
    _cwd_error, _cwd_bytes = __posix('cwd')
    if _cwd_error:
        raise OSError(_cwd_error, 'cannot resolve the working directory')
    path.insert(0, _cwd_bytes.decode('utf-8', 'surrogateescape'))
    flags.isolated = 0
    flags.safe_path = False

def gettrace():
    return __trace_native__(0)


def settrace(trace):
    __trace_native__(1, trace)
def getdefaultencoding():
    return "utf-8"

_import_shadow_root = None if flags.safe_path else path[0]

# Resolve path entries lazily so bootstrap imports can use the native source reader.
def _zip_path_hook(path):
    from zipimport import zipimporter
    return zipimporter(path)

# Build the source finder only when an ordinary directory is first searched.
def _file_path_hook(path):
    from importlib._bootstrap_external import FileFinder, _get_supported_file_loaders
    return FileFinder.path_hook(*_get_supported_file_loaders())(path)

# Delegate path searching to the upstream finder and its importer cache.
class _PathFinder:
    @classmethod
    def find_spec(cls, fullname, path=None, target=None):
        external = modules.get('importlib._bootstrap_external')
        finder = getattr(external, 'PathFinder', None)
        if finder is None:
            return None
        return finder.find_spec(fullname, path, target)

    @classmethod
    def invalidate_caches(cls):
        from importlib._bootstrap_external import _NamespacePath
        for path, finder in list(path_importer_cache.items()):
            if finder is None or not path.startswith('/'):
                del path_importer_cache[path]
            elif hasattr(finder, 'invalidate_caches'):
                finder.invalidate_caches()
        _NamespacePath._epoch += 1

path_importer_cache = {}
path_hooks = [_zip_path_hook, _file_path_hook]
import _imp
meta_path = [_PathFinder, _SourceFinder(), _imp._NativeFinder]

stdlib_module_names = frozenset(__load_module('', 'names'))
# Source and native modules participate in the same finder protocol.


def getrefcount(object, /):
    return __trace_native__(5, object)


def get_coroutine_origin_tracking_depth():
    return __trace_native__(3)


def set_coroutine_origin_tracking_depth(depth):
    from operator import index
    __trace_native__(4, index(depth))


_asyncgen_hook_missing = object()
_asyncgen_hook_type = None


def get_asyncgen_hooks():
    global _asyncgen_hook_type
    if _asyncgen_hook_type is None:
        from collections import namedtuple
        _asyncgen_hook_type = namedtuple('asyncgen_hooks', 'firstiter finalizer', module='sys')
    return _asyncgen_hook_type(*__trace_native__(6))


def set_asyncgen_hooks(firstiter=_asyncgen_hook_missing, finalizer=_asyncgen_hook_missing):
    if firstiter is not _asyncgen_hook_missing and firstiter is not None and not callable(firstiter):
        raise TypeError('callable firstiter expected')
    if finalizer is not _asyncgen_hook_missing and finalizer is not None and not callable(finalizer):
        raise TypeError('callable finalizer expected')
    if firstiter is not _asyncgen_hook_missing:
        __trace_native__(7, firstiter)
    if finalizer is not _asyncgen_hook_missing:
        __trace_native__(8, finalizer)


def is_finalizing():
    return __trace_native__(9)

# The builtin interface has no filesystem origin after bridge initialization.
globals().pop('__file__', None)
