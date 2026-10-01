# Spec and module helpers from CPython 3b564385e4c9; PSF License.
from ._bootstrap import module_from_spec, spec_from_loader, _resolve_name
from ._bootstrap_external import spec_from_file_location, MAGIC_NUMBER

def resolve_name(name, package):
    if not name.startswith('.'):
        return name
    if not package:
        raise ImportError('no package specified for ' + repr(name))
    level = len(name) - len(name.lstrip('.'))
    return _resolve_name(name[level:], package, level)

def find_spec(name, package=None):
    import sys
    from .machinery import PathFinder
    name = resolve_name(name, package)
    if name in sys.modules:
        module = sys.modules[name]
        if module is None:
            return None
        spec = getattr(module, '__spec__', None)
        if spec is None:
            raise ValueError(name + '.__spec__ is None')
        return spec
    path = None
    if '.' in name:
        from . import import_module
        parent = import_module(name.rpartition('.')[0])
        path = parent.__path__
    return PathFinder.find_spec(name, path)
