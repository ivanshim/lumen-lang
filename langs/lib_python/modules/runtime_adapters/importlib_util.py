
# The native reader answers `import` itself and leaves a module it read
# without the CPython spec the reference keeps beside every module. A
# caller that asks after a module's loader (pkgutil.get_data, runpy and
# the like) is given one built from the file and package path that the
# reader bound, and the module keeps it for the next ask.
import importlib._bootstrap as _bootstrap_for_specs
import importlib._bootstrap_external as _bootstrap_external_for_specs

_find_spec_without_native_specs = find_spec


def _spec_from_native_module(name, module):
    search = getattr(module, '__path__', None)
    origin = getattr(module, '__file__', None)
    loader = None
    if origin is not None:
        loader = _bootstrap_external_for_specs.SourceFileLoader(name, origin)
    spec = _bootstrap_for_specs.ModuleSpec(name, loader, origin=origin)
    if search is not None:
        spec.submodule_search_locations = list(search)
    return spec


def find_spec(name, package=None):
    fullname = resolve_name(name, package) if name.startswith('.') else name
    module = sys.modules.get(fullname)
    if module is None:
        # A name the native reader can bring in is read in here so that
        # the spec describes a loader that has the module's own file.
        try:
            module = __load_module(fullname)
        except BaseException:
            module = None
    if module is not None:
        spec = getattr(module, '__spec__', None)
        if spec is None:
            spec = _spec_from_native_module(fullname, module)
            try:
                module.__spec__ = spec
            except BaseException:
                pass
        return spec
    return _find_spec_without_native_specs(name, package)
