from types import ModuleType

def cache_from_source(path, debug_override=None, *, optimization=None):
    raise NotImplementedError('this interpreter does not write CPython bytecode')

def module_from_spec(spec):
    if spec.loader is not None:
        raise NotImplementedError('custom import loaders are not supported')
    module = ModuleType(spec.name)
    module.__loader__ = spec.loader
    module.__package__ = spec.parent
    module.__spec__ = spec
    if spec.submodule_search_locations is not None:
        module.__path__ = spec.submodule_search_locations
    if spec.has_location:
        module.__file__ = spec.origin
        if spec.cached is not None:
            module.__cached__ = spec.cached
    return module
