# Host details which the present numeric and object model can honour.
argv = __program_namespace()['__program_argv']
maxsize = 9223372036854775807
version_info = (3, 14, 0, 'final', 0)
platform = 'linux'
# The cache is refreshed after imports; editing this view does not yet
# alter the loader's stored namespaces.
modules = {}
# Stub: this is the advertised limit; the kernel does not yet count calls.
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
    bits_per_digit = 32
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
    # A stored limit would pretend to govern calls which it cannot govern.
    raise 'NotImplementedError: setting the recursion limit is not supported'

class _Output:
    def write(self, text):
        return __stream_write(text, False)

    def flush(self):
        pass

class _Error:
    def write(self, text):
        return __stream_write(text, True)

    def flush(self):
        pass

class _Input:
    def read(self, size=-1):
        return __stream_read(size, False)

    def readline(self, size=-1):
        return __stream_read(size, True)

stdout = _Output()
stderr = _Error()
stdin = _Input()
__stdout__ = stdout
__stderr__ = stderr
__stdin__ = stdin

def _input(prompt=''):
    print(prompt, end='', flush=True)
    line = stdin.readline()
    if not isinstance(line, str):
        raise 'TypeError: readline must return a string'
    if line == '':
        raise 'EOFError: EOF when reading a line'
    if line[-1:] == '\n':
        return line[:-1]
    return line

# Stub: the direct host spelling still raises a complaint. Numeric
# process exit status and catchable SystemExit await exception support.
def exit(status=0):
    sys.exit(status)
