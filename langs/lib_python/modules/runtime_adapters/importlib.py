# Register the source-file hook used by the public PathFinder.
from ._bootstrap_external import FileFinder, SourceFileLoader, SOURCE_SUFFIXES
sys.path_hooks.append(FileFinder.path_hook((SourceFileLoader, SOURCE_SUFFIXES)))

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

# Complete metadata for sources imported while bootstrap was initializing.
for _name, _module in list(sys.modules.items()):
    if getattr(_module, '__spec__', None) is None and getattr(_module, '__file__', None):
        _imp._initialize_source_module(_module, _name, _module.__file__)
del _name, _module
