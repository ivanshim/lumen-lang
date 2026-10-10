# Names of the standard-library modules registered in the source manifest.
stdlib_module_names = frozenset(('__future__', '_bisect', '_blake2', '_buffer', '_codec_idna', '_codec_names', '_codecs', '_collections', '_collections_abc', '_colorize', '_contextvars', '_csv', '_decimal', '_digest', '_encoding_aliases', '_heapq', '_imp', '_io', '_json', '_locale', '_markupbase', '_md5', '_namedtuple', '_object_reduce', '_operator', '_ordering', '_os_pathlike', '_py_abc', '_pydatetime', '_pydecimal', '_pyio', '_pylong', '_pyrepl', '_random', '_random_seed', '_sha1', '_sha2', '_sha3', '_sre', '_sre_validation', '_stat', '_statistics', '_string', '_strptime', '_struct', '_template_core', '_thread', '_tokenize', '_types', '_warnings', '_weakref', '_weakrefset', 'abc', 'annotationlib', 'argparse', 'array', 'ast', 'atexit', 'base64', 'binascii', 'bisect', 'builtins', 'calendar', 'cmath', 'codecs', 'codeop', 'collections', 'colorsys', 'compression', 'concurrent', 'configparser', 'contextlib', 'contextvars', 'copy', 'copyreg', 'csv', 'ctypes', 'dataclasses', 'datetime', 'decimal', 'difflib', 'dis', 'doctest', 'encodings', 'enum', 'errno', 'fnmatch', 'fractions', 'functools', 'gc', 'genericpath', 'getopt', 'gettext', 'glob', 'graphlib', 'gzip', 'hashlib', 'heapq', 'hmac', 'html', 'http', 'importlib', 'inspect', 'io', 'ipaddress', 'itertools', 'json', 'keyword', 'linecache', 'locale', 'logging', 'marshal', 'math', 'ntpath', 'numbers', 'operator', 'optparse', 'os', 'pathlib', 'pdb', 'pickle', 'pkgutil', 'platform', 'posix', 'posixpath', 'pprint', 'pty', 'py_compile', 'pydoc', 'queue', 'quopri', 'random', 're', 'reprlib', 'sched', 'secrets', 'select', 'shlex', 'shutil', 'signal', 'socket', 'sre_compile', 'sre_constants', 'sre_parse', 'stat', 'statistics', 'string', 'struct', 'subprocess', 'sys', 'sysconfig', 'tempfile', 'textwrap', 'threading', 'time', 'timeit', 'token', 'tokenize', 'traceback', 'types', 'typing', 'unicodedata', 'unittest', 'urllib', 'warnings', 'weakref', 'zlib'))

# Read the host's string-to-string option map without importing the JSON package.
_xoptions = {}
_options_environment = __posix('environ')[1]
_options_text = _options_environment.get(b'LUMEN_PYTHON_XOPTIONS', b'{}').decode('utf-8')
_options_cursor = 1
while _options_cursor < len(_options_text) and _options_text[_options_cursor] != '}':
    _option_name, _options_cursor = __json_string_scan(_options_text, _options_cursor + 1)
    _option_value, _options_cursor = __json_string_scan(_options_text, _options_cursor + 2)
    _xoptions[_option_name] = True if _option_value == 'True' else _option_value
    if _options_text[_options_cursor:_options_cursor + 1] == ',':
        _options_cursor += 1

# Combine CLI warning options with the native startup parser.
_warning_text = _options_environment.get(b'LUMEN_PYTHON_WARNOPTIONS', b'[]').decode('utf-8')
_warning_cursor = 1
while _warning_cursor < len(_warning_text) and _warning_text[_warning_cursor] != ']':
    _warning_option, _warning_cursor = __json_string_scan(_warning_text, _warning_cursor + 1)
    warnoptions.append(_warning_option)
    if _warning_text[_warning_cursor:_warning_cursor + 1] == ',':
        _warning_cursor += 1

flags.bytes_warning = int(_options_environment.get(b'LUMEN_PYTHON_BYTES_WARNING', b'0'))
if _options_environment.get(b'LUMEN_PYTHON_NO_SITE'):
    flags.no_site = 1

del _options_environment

# Native standard text streams use the Rust host's UTF-8 byte representation.
_StandardStream.encoding = 'utf-8'

# Read the live teardown state after normal exit callbacks have completed.
def is_finalizing():
    return __program_namespace()['__program_finalizing']

# Startup flags read the host environment without a late module import.
def _environment(name):
    environment = _host_environment('environ')[1]
    value = environment.get(name)
    if value is None:
        value = environment.get(name.encode())
    return value.decode() if isinstance(value, bytes) else value
