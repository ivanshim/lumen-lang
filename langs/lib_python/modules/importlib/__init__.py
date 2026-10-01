# Import helpers using the kernels' import execution.
import sys
from . import _bootstrap

def import_module(name, package=None):
    if name.startswith('.'):
        if not package:
            raise TypeError("the 'package' argument is required to perform a relative import")
        level = len(name) - len(name.lstrip('.'))
        name = _bootstrap._resolve_name(name[level:], package, level)
    return __import__(name, fromlist=['*'])

def invalidate_caches():
    for finder in getattr(sys, 'meta_path', ()):
        if hasattr(finder, 'invalidate_caches'):
            finder.invalidate_caches()

from . import _bootstrap_external
_bootstrap._bootstrap_external = _bootstrap_external
