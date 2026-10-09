# Bringing a module in is the reader's own work. This module names the
# parts of that machinery a program is allowed to ask after, and the
# one call that settles any finder state. The reader reads directories
# as it goes and keeps nothing of their listing between imports, so
# there is never a stale cache to clear and the ask is answered with
# nothing to do. The naming of a bytecode cache stands in
# importlib.util, beside this module.


def invalidate_caches():
    for finder in sys.meta_path:
        invalidate = getattr(finder, 'invalidate_caches', None)
        if invalidate is not None:
            invalidate()

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

def _invalidate_path_caches():
    for path, finder in list(sys.path_importer_cache.items()):
        if finder is None or not _bootstrap_external._path_isabs(path):
            del sys.path_importer_cache[path]
        elif hasattr(finder, 'invalidate_caches'):
            finder.invalidate_caches()
    _bootstrap_external._NamespacePath._epoch += 1
    # An unloaded optional metadata module has no discovery cache to clear.
    metadata = sys.modules.get('importlib.metadata')
    if metadata is not None:
        metadata.MetadataPathFinder.invalidate_caches()

_bootstrap_external.PathFinder.invalidate_caches = staticmethod(_invalidate_path_caches)

# Bootstrap discovery around the native source loader, including namespace packages.
for _index, _finder in enumerate(sys.meta_path):
    if type(_finder) is sys._SourceFinder:
        sys.meta_path[_index] = _imp._NativeFinder
sys.meta_path.append(_bootstrap_external.PathFinder)
sys.path_hooks.append(_bootstrap_external.FileFinder.path_hook(
    (_bootstrap_external.SourceFileLoader, _bootstrap_external.SOURCE_SUFFIXES)))
