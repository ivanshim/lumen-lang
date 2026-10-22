# The modules built into the interpreter.  Their source travels inside the
# binary in this module; a frozen module is compiled the first time its code
# object is asked for, and a name not yet compiled still answers with its
# package flag and original name.  The table carries the reference's own
# sample modules, which exist only as frozen modules, and the two importlib
# spellings of the bootstrap modules.
_FROZEN_SOURCES = {
    '__hello__': r'''initialized = True

class TestFrozenUtf8_1:
    """\u00b6"""

class TestFrozenUtf8_2:
    """\u03c0"""

class TestFrozenUtf8_4:
    """\U0001f600"""

def main():
    print("Hello world!")

if __name__ == '__main__':
    main()
''',
    '__phello__': r'''initialized = True

def main():
    print("Hello world!")

if __name__ == '__main__':
    main()
''',
    '__phello__.spam': r'''initialized = True

def main():
    print("Hello world!")

if __name__ == '__main__':
    main()
''',
    '__phello__.ham': r'''initialized = True

def main():
    print("Hello world!")

if __name__ == '__main__':
    main()
''',
    '__phello__.ham.eggs': r'''initialized = True

def main():
    print("Hello world!")

if __name__ == '__main__':
    main()
''',
    '__hello_only__': r'''initialized = True
print("Hello world!")
''',
}
_FROZEN_PACKAGES = frozenset(('__phello__', '__phello__.ham'))
# A frozen module built from a loose source file has no original name.
_FROZEN_ORIGINLESS = frozenset(('__hello_only__',))
_FROZEN_ALIASES = {
    '__hello_alias__': ('__hello__', False),
    '__phello_alias__': ('__hello__', True),
    '_frozen_importlib': ('importlib._bootstrap', False),
    '_frozen_importlib_external': ('importlib._bootstrap_external', False),
}
_FROZEN_CODES = {}
_frozen_override = 0


def _override_frozen_modules_for_tests(override):
    global _frozen_override
    if override not in (-1, 0, 1):
        raise ValueError('invalid frozen module override')
    _frozen_override = override


def _frozen_entry(name):
    if _frozen_override == -1:
        return None
    if name in _FROZEN_ALIASES:
        original, package = _FROZEN_ALIASES[name]
        return original, package, original
    if name.endswith('.__init__'):
        original = name[:-len('.__init__')]
        if original in _FROZEN_PACKAGES:
            return original, False, '<' + original
        return None
    if name in _FROZEN_SOURCES:
        original = None if name in _FROZEN_ORIGINLESS else name
        return name, name in _FROZEN_PACKAGES, original
    return None


def _module_source(name):
    # A bootstrap module's own source is the closest thing to the code a
    # frozen table would hold for it, so it is compiled only when asked for.
    import sys
    module = sys.modules.get(name)
    if module is None:
        return None
    loader = getattr(module, '__loader__', None)
    fetch = getattr(loader, 'get_source', None)
    if fetch is not None:
        try:
            return fetch(name)
        except (ImportError, AttributeError):
            pass
    filename = getattr(module, '__file__', None)
    if not filename:
        return None
    try:
        with open(filename, 'r', encoding='utf-8') as stream:
            return stream.read()
    except OSError:
        return None


def _frozen_code(key, asked):
    code = _FROZEN_CODES.get(asked)
    if code is not None:
        return code
    source = _FROZEN_SOURCES.get(key)
    if source is None:
        source = _module_source(key)
    if source is None:
        raise ImportError(f"No such frozen object named {asked!r}", name=asked)
    code = compile(source, '<frozen ' + key + '>', 'exec')
    _FROZEN_CODES[asked] = code
    return code


def is_frozen(name):
    return _frozen_entry(name) is not None


def is_frozen_package(name):
    entry = _frozen_entry(name)
    if entry is None:
        raise ImportError(f"No such frozen object named {name!r}", name=name)
    return entry[1]


# This interpreter carries source modules; the frozen table above is its own.
def find_frozen(name, withdata=False):
    entry = _frozen_entry(name)
    if entry is None:
        return None
    key, package, original = entry
    data = None
    if withdata:
        import marshal
        data = memoryview(marshal.dumps(_frozen_code(key, name)))
    return (data, package, original)

def _override_multi_interp_extensions_check(override):
    raise NotImplementedError('subinterpreters and binary extensions are not supported')

# Compiled-in source modules share the interpreter's native import loader.
def is_builtin(name):
    if name in ('sys', 'builtins'):
        return -1
    return 1 if name in ('_imp', '_thread', '_weakref', '_io', 'posix', 'marshal', '_warnings') else 0


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


def get_frozen_object(name, data=None):
    if data is None:
        entry = _frozen_entry(name)
        if entry is None:
            raise ImportError(f"No such frozen object named {name!r}", name=name)
        return _frozen_code(entry[0], name)
    import marshal
    import types
    try:
        code = marshal.loads(data)
    except (EOFError, ValueError, TypeError, UnicodeError):
        raise ImportError(f"Frozen object named {name!r} is invalid", name=name) from None
    if not isinstance(code, types.CodeType):
        raise ImportError(f"Frozen object named {name!r} is invalid", name=name)
    return code
