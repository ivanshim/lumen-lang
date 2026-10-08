# Bringing a module in is the reader's own work. This module names the
# parts of that machinery a program is allowed to ask after, and the
# one call that settles any finder state. The reader reads directories
# as it goes and keeps nothing of their listing between imports, so
# there is never a stale cache to clear and the ask is answered with
# nothing to do. The naming of a bytecode cache stands in
# importlib.util, beside this module.


def invalidate_caches():
    return None

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

# The reader answers `import` itself; the reference machinery a library
# asks after is installed here beside it, once, so `path_hooks`,
# `meta_path` and `path_importer_cache` hold the finders CPython's own
# FileFinder, PathFinder and zipimporter provide.
import sys as _machinery_sys

if not getattr(_machinery_sys, '_lumen_importers_ready', False):
    try:
        import importlib._bootstrap as _machinery_bootstrap
        import importlib._bootstrap_external as _machinery_external
    except ImportError:
        pass
    else:
        _machinery_external._install(_machinery_bootstrap)
        _machinery_sys._lumen_importers_ready = True
