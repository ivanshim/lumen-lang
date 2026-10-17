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

def get_frozen_object(name, data=None):
    _frozen_info(name)
    if data is not None:
        import marshal
        return marshal.loads(data)
    return compile(__load_module(name, 'source'), '<frozen ' + name + '>', 'exec')

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

    @staticmethod
    def get_source(fullname):
        return __loader_source(fullname)

# Cached code carries the source location through its nested definitions.
def _fix_co_filename(code, path):
    return _code_filename(code, path)
