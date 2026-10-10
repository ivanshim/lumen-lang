# Imports use the native reader; cache invalidation reaches registered finders.


def invalidate_caches():
    import sys
    for finder in list(sys.meta_path):
        if hasattr(finder, "invalidate_caches"):
            finder.invalidate_caches()

# The utility part lives in importlib.util; the name below stands ready
# for the embedded alias that brings that module in beside this one.
util = None


def import_module(name, package=None):
    # A name brought in by the reader, spelled the way the reference
    # spells a relative name against its package.
    if name.startswith('.'):
        if not package:
            raise TypeError("the 'package' argument is required to perform a relative import")
        level = len(name) - len(name.lstrip('.'))
        parts = package.rsplit('.', level - 1)
        if len(parts) < level:
            raise ImportError('attempted relative import beyond top-level package')
        remainder = name[level:]
        name = parts[0] + '.' + remainder if remainder else parts[0]
    return __load_module(name)

# Install the standard source finders after the bootstrap modules are ready.
import sys as _sys
_sys.meta_path = getattr(_sys, "meta_path", [_bootstrap.BuiltinImporter, _bootstrap.FrozenImporter])
_sys.path_hooks = getattr(_sys, "path_hooks", [])
_sys.path_importer_cache = getattr(_sys, "path_importer_cache", {})
_bootstrap_external._install(_bootstrap)

# Describe native source imports with the same file-loader metadata.
def _native_spec(module):
    name = module.__name__
    spec = _bootstrap.BuiltinImporter.find_spec(name)
    filename = getattr(module, '__file__', None)
    if spec is None and filename is not None:
        spec = _bootstrap_external.spec_from_file_location(name, filename)
    if spec is not None:
        _bootstrap._init_module_attrs(spec, module, override=True)

from _runtime_import import SourceLoader as _BootstrapSourceLoader
for _module in list(_sys.modules.values()):
    if isinstance(_module, type(_sys)):
        _spec = getattr(_module, '__spec__', None)
        if _spec is None or isinstance(_spec.loader, _BootstrapSourceLoader):
            _native_spec(_module)

# Let installed finders handle namespaces and other non-source path entries.
def _native_import(name):
    parent_name, _, child = name.rpartition('.')
    parent = _sys.modules[parent_name] if parent_name else None
    path = parent.__path__ if parent is not None else None
    spec = _bootstrap._find_spec(name, path)
    if spec is None:
        raise ModuleNotFoundError(f"No module named {name!r}", name=name)
    module = _bootstrap._load(spec)
    if parent is not None:
        setattr(parent, child, module)
    return module
