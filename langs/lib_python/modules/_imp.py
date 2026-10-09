# The source loader has no frozen modules or binary extensions.
def _override_frozen_modules_for_tests(override):
    if override not in (-1, 0, 1):
        raise ValueError('invalid frozen module override')

def _override_multi_interp_extensions_check(override):
    raise NotImplementedError('subinterpreters and binary extensions are not supported')

# Compiled-in source modules share the interpreter's native import loader.
def is_builtin(name):
    if name in ('sys', 'builtins'):
        return -1
    return 1 if name in ('_imp', '_thread', '_warnings', '_weakref', '_io', 'posix', 'marshal') else 0

def is_frozen(name):
    return False

def create_builtin(spec):
    if not is_builtin(spec.name):
        return None
    return __load_module(spec.name)

def exec_builtin(module):
    return 0

def extension_suffixes():
    return []

# Marker of the active kernel compiled-code cache ABI.
pyc_magic_number_token = _code_abi()[1]
check_hash_based_pycs = 'default'

_import_lock_depth = 0

def acquire_lock():
    global _import_lock_depth
    _import_lock_depth += 1

def release_lock():
    global _import_lock_depth
    if not _import_lock_depth:
        raise RuntimeError('not holding the import lock')
    _import_lock_depth -= 1

def lock_held():
    return bool(_import_lock_depth)

class _NativeFinder:
    @classmethod
    def find_spec(cls, fullname, path=None, target=None):
        if not __load_module(fullname, False):
            return None
        from importlib._bootstrap import ModuleSpec
        origin = __load_module(fullname, True)
        spec = ModuleSpec(fullname, _NativeLoader, origin=origin, is_package=bool(origin and origin.endswith('__init__.py')))
        spec.has_location = origin is not None
        return spec

class _NativeLoader:
    @staticmethod
    def create_module(spec):
        return __load_module(spec.name)
    @staticmethod
    def exec_module(module):
        # The source-backed native create operation initializes the namespace.
        pass

# Cached code carries the source location through its nested definitions.
def _fix_co_filename(code, path):
    return _code_filename(code, path)

# PEP 552 uses a deterministic SipHash-1-3 keyed by the bytecode magic token.
def source_hash(key, source, /):
    import operator
    key = operator.index(key)
    if key < -0x8000000000000000 or key > 0x7fffffffffffffff:
        raise OverflowError('Python int too large to convert to C long')
    key &= 0xffffffffffffffff
    source = memoryview(source).tobytes()
    mask = 0xffffffffffffffff
    def rotate(word, bits):
        return ((word << bits) | (word >> (64 - bits))) & mask
    def round_once(words):
        a, b, c, d = words
        a = (a + b) & mask
        c = (c + d) & mask
        b = rotate(b, 13) ^ a
        d = rotate(d, 16) ^ c
        a = rotate(a, 32)
        c = (c + b) & mask
        a = (a + d) & mask
        b = rotate(b, 17) ^ c
        d = rotate(d, 21) ^ a
        c = rotate(c, 32)
        return a, b, c, d
    words = (key ^ 0x736f6d6570736575, 0x646f72616e646f6d,
             key ^ 0x6c7967656e657261, 0x7465646279746573)
    end = len(source) - len(source) % 8
    for offset in range(0, end, 8):
        block = int.from_bytes(source[offset:offset + 8], 'little')
        a, b, c, d = words
        a, b, c, d = round_once((a, b, c, d ^ block))
        words = (a ^ block, b, c, d)
    tail = ((len(source) & 255) << 56) | int.from_bytes(source[end:], 'little')
    a, b, c, d = words
    a, b, c, d = round_once((a, b, c, d ^ tail))
    words = (a ^ tail, b, c ^ 255, d)
    for _ in range(3):
        words = round_once(words)
    a, b, c, d = words
    return (a ^ b ^ c ^ d).to_bytes(8, 'little')
