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

# Bytecode marker from the pinned CPython pycore_magic_number.h.
pyc_magic_number_token = 168627833
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
