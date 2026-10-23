# The source loader has no binary extensions.  The few library modules that
# CPython ships frozen are reported as frozen; their table and the override
# live beside the meta path entry that serves them.
import _runtime_import as _frozen

def _override_frozen_modules_for_tests(override):
    if override not in (-1, 0, 1):
        raise ValueError('invalid frozen module override')
    _frozen.frozen_override = override

def _frozen_module_names():
    return list(_frozen.FROZEN_MODULES)

def find_frozen(name, *, withdata=False):
    if not isinstance(name, str):
        raise TypeError('find_frozen() argument 1 must be str, not ' + type(name).__name__)
    if _frozen.frozen_override < 0 or name not in _frozen.FROZEN_MODULES:
        return None
    ispkg, origname = _frozen.FROZEN_MODULES[name]
    data = None
    if withdata:
        import marshal
        data = marshal.dumps(get_frozen_object(name))
    return (data, ispkg, origname)

def _frozen_info(name):
    info = find_frozen(name)
    if info is None:
        raise ImportError('No such frozen object named ' + repr(name), name=name)
    return info

def is_frozen_package(name):
    return _frozen_info(name)[1]

def init_frozen(name):
    _frozen_info(name)
    from importlib._bootstrap import FrozenImporter, _load_unlocked
    return _load_unlocked(FrozenImporter.find_spec(name))

def _override_multi_interp_extensions_check(override):
    raise NotImplementedError('subinterpreters and binary extensions are not supported')

# Compiled-in source modules share the interpreter's native import loader.
def is_builtin(name):
    if name in ('sys', 'builtins'):
        return -1
    return 1 if name in ('_imp', '_thread', '_weakref', '_io', 'posix', 'marshal', '_warnings') else 0

def is_frozen(name):
    return find_frozen(name) is not None

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
        from importlib._bootstrap_external import spec_from_file_location
        if is_builtin(fullname):
            from importlib._bootstrap import BuiltinImporter, spec_from_loader
            return spec_from_loader(fullname, BuiltinImporter)
        origin = __load_module(fullname, True)
        return spec_from_file_location(fullname, origin) if origin else None

    # Clear directory and distribution caches without importing metadata eagerly.
    @staticmethod
    def invalidate_caches():
        import sys
        from importlib._bootstrap_external import PathFinder
        PathFinder.invalidate_caches()
        metadata = sys.modules.get('importlib.metadata')
        if metadata is not None:
            metadata.MetadataPathFinder.invalidate_caches()

    @staticmethod
    def find_distributions(*args, **kwargs):
        import sys
        from importlib._bootstrap_external import PathFinder
        # The standard finder already searches these same distribution paths.
        if any(finder is PathFinder for finder in sys.meta_path):
            return iter(())
        from importlib.metadata import MetadataPathFinder
        return MetadataPathFinder.find_distributions(*args, **kwargs)

class _NativeLoader:
    @staticmethod
    def create_module(spec):
        return __load_module(spec.name)
    @staticmethod
    def exec_module(module):
        # The source-backed native create operation initializes the namespace.
        pass

    @staticmethod
    def get_source(fullname):
        return __loader_source(fullname)

# Cached code carries the source location through its nested definitions.
def _fix_co_filename(code, path):
    return _code_filename(code, path)

# Initialize native source modules with the public import machinery.
def _initialize_source_module(module, name, filename):
    from importlib._bootstrap_external import spec_from_file_location
    from importlib._bootstrap import _init_module_attrs
    if is_builtin(name):
        from importlib._bootstrap import BuiltinImporter, spec_from_loader
        spec = spec_from_loader(name, BuiltinImporter)
    else:
        spec = spec_from_file_location(name, filename)
    if spec is not None:
        _init_module_attrs(spec, module, override=True)

# Mix one SipHash round with unsigned 64-bit arithmetic.
def _source_hash_round(v):
    mask = (1 << 64) - 1
    v[0] = (v[0] + v[1]) & mask
    v[2] = (v[2] + v[3]) & mask
    v[1] = ((v[1] << 13) | (v[1] >> 51)) & mask ^ v[0]
    v[3] = ((v[3] << 16) | (v[3] >> 48)) & mask ^ v[2]
    v[0] = ((v[0] << 32) | (v[0] >> 32)) & mask
    v[2] = (v[2] + v[1]) & mask
    v[0] = (v[0] + v[3]) & mask
    v[1] = ((v[1] << 17) | (v[1] >> 47)) & mask ^ v[2]
    v[3] = ((v[3] << 21) | (v[3] >> 43)) & mask ^ v[0]
    v[2] = ((v[2] << 32) | (v[2] >> 32)) & mask

# Hash source bytes with CPython's magic-number-keyed SipHash-1-3.
def source_hash(key, source, /):
    import operator
    key = operator.index(key)
    if key < -(1 << 63) or key >= 1 << 63:
        raise OverflowError('Python int too large to convert to C long')
    key &= (1 << 64) - 1
    view = memoryview(source)
    if not view.c_contiguous:
        raise BufferError('memoryview: underlying buffer is not C-contiguous')
    data = view.tobytes()
    v = [key ^ 0x736f6d6570736575, 0x646f72616e646f6d,
         key ^ 0x6c7967656e657261, 0x7465646279746573]
    end = len(data) - len(data) % 8
    for at in range(0, end, 8):
        word = int.from_bytes(data[at:at + 8], 'little')
        v[3] ^= word
        _source_hash_round(v)
        v[0] ^= word
    tail = int.from_bytes(data[end:], 'little') | ((len(data) & 255) << 56)
    v[3] ^= tail
    _source_hash_round(v)
    v[0] ^= tail
    v[2] ^= 255
    for _ in range(3):
        _source_hash_round(v)
    return (v[0] ^ v[1] ^ v[2] ^ v[3]).to_bytes(8, 'little')


def create_dynamic(spec, file=None):
    import os
    name = spec.name
    origin = os.fspath(spec.origin)
    if '\x00' in name or (b'\x00' in origin if isinstance(origin, bytes) else '\x00' in origin):
        raise ValueError('embedded null character')
    raise ImportError('dynamic extension loading is not supported', name=name, path=origin)


# Compile bundled frozen sources or validate explicitly supplied marshalled code.
def get_frozen_object(name, data=None):
    if data is None:
        _frozen_info(name)
        return compile(__load_module(name, 'source'), '<frozen ' + name + '>', 'exec')
    import marshal
    import types
    try:
        code = marshal.loads(data)
    except (EOFError, ValueError, TypeError, UnicodeError):
        raise ImportError(f"Frozen object named {name!r} is invalid", name=name) from None
    if not isinstance(code, types.CodeType):
        raise ImportError(f"Frozen object named {name!r} is invalid", name=name)
    return code
