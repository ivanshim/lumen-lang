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
    return 1 if name in ('_imp', '_thread', '_weakref', '_io', 'posix', 'marshal', '_warnings') else 0

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

# This interpreter carries source modules, with no frozen code table.
def find_frozen(name, withdata=False):
    return None

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

# The directories on a search path that make a name a namespace
# portion, as the reference finder counts them: a matching directory
# with no __init__.py in it, kept in search-path order.
def _namespace_portion_spec(name, search):
    import os
    from importlib._bootstrap import ModuleSpec
    leaf = name.rpartition('.')[2]
    portions = []
    for entry in search:
        if not isinstance(entry, str):
            continue
        folder = os.path.join(entry, leaf)
        if os.path.isdir(folder) and not os.path.exists(os.path.join(folder, '__init__.py')):
            portions.append(os.path.abspath(folder))
    if not portions:
        return None
    spec = ModuleSpec(name, None, origin=None, is_package=True)
    spec.submodule_search_locations = portions
    return spec

# The bootstrap asks again when the parent search path has moved or the
# finder caches were cleared: answer as the path finder does, so a regular
# package or module found on the path wins and the bootstrap keeps its path.
def _namespace_path_finder(name, path):
    from importlib._bootstrap_external import PathFinder
    return PathFinder._get_spec(name, path)

# Build the module a name's directories answer for when no source is
# found: a namespace package, whose search path and loader are the
# bootstrap's own, and whose body is nothing.
def _initialize_namespace_module(name):
    import sys
    from importlib._bootstrap import _new_module, _init_module_attrs
    from importlib._bootstrap_external import _NamespacePath
    parent = name.rpartition('.')[0]
    if parent:
        holder = sys.modules.get(parent)
        if holder is None:
            return None
        search = list(holder.__path__)
    else:
        search = list(sys.path)
    spec = _namespace_portion_spec(name, search)
    if spec is None:
        return None
    spec.submodule_search_locations = _NamespacePath(
        name, list(spec.submodule_search_locations), _namespace_path_finder)
    module = _new_module(name)
    _init_module_attrs(spec, module, override=True)
    return module

# Read a namespace path through its own iterator, which is what asks
# the bootstrap to recompute the portions from the parent's search path.
def _refresh_namespace_path(path):
    list(path)

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


def get_frozen_object(name, data=None):
    if data is None:
        raise ImportError(f"No such frozen object named {name!r}", name=name)
    import marshal
    import types
    try:
        code = marshal.loads(data)
    except (EOFError, ValueError, TypeError, UnicodeError):
        raise ImportError(f"Frozen object named {name!r} is invalid", name=name) from None
    if not isinstance(code, types.CodeType):
        raise ImportError(f"Frozen object named {name!r} is invalid", name=name)
    return code
