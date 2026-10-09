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

# Bootstrap discovery around the native source loader, including namespace packages.
sys.meta_path.extend((_imp._NativeFinder, _bootstrap_external.PathFinder))
sys.path_hooks.append(_bootstrap_external.FileFinder.path_hook(
    (_bootstrap_external.SourceFileLoader, _bootstrap_external.SOURCE_SUFFIXES)))
