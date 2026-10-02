# Import through the interpreter's Python module loader and its cache.
import sys

def import_module(name, package=None):
    if name.startswith('.'):
        if not package:
            raise TypeError("the 'package' argument is required to perform a relative import")
        level = len(name) - len(name.lstrip('.'))
        parts = package.rsplit('.', level - 1)
        if len(parts) < level:
            raise ImportError('attempted relative import beyond top-level package')
        tail = name[level:]
        name = parts[0] + ('.' + tail if tail else '')
    return __load_module(name)

def invalidate_caches():
    for finder in getattr(sys, 'meta_path', ()):
        if hasattr(finder, 'invalidate_caches'):
            finder.invalidate_caches()
