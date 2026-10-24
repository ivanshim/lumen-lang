_load_module = __load_module
# ModuleSpec adapted from: CPython v3.14.8 Lib/importlib/_bootstrap.py, PSF license.
class ModuleSpec:
    """The specification for a module, used for loading.

    A module's spec is the source for information about the module.  For
    data associated with the module, including source, use the spec's
    loader.

    `name` is the absolute name of the module.  `loader` is the loader
    to use when loading the module.  `parent` is the name of the
    package the module is in.  The parent is derived from the name.

    `is_package` determines if the module is considered a package or
    not.  On modules this is reflected by the `__path__` attribute.

    `origin` is the specific location used by the loader from which to
    load the module, if that information is available.  When filename is
    set, origin will match.

    `has_location` indicates that a spec's "origin" reflects a location.
    When this is True, `__file__` attribute of the module is set.

    `cached` is the location of the cached bytecode file, if any.  It
    corresponds to the `__cached__` attribute.

    `submodule_search_locations` is the sequence of path entries to
    search when importing submodules.  If set, is_package should be
    True--and False otherwise.

    Packages are simply modules that (may) have submodules.  If a spec
    has a non-None value in `submodule_search_locations`, the import
    system will consider modules loaded from the spec as packages.

    Only finders (see importlib.abc.MetaPathFinder and
    importlib.abc.PathEntryFinder) should modify ModuleSpec instances.

    """

    def __init__(self, name, loader, *, origin=None, loader_state=None,
                 is_package=None):
        self.name = name
        self.loader = loader
        self.origin = origin
        self.loader_state = loader_state
        self.submodule_search_locations = [] if is_package else None
        self._uninitialized_submodules = []

        # file-location attributes
        self._set_fileattr = False
        self._cached = None

    def __repr__(self):
        args = [f'name={self.name!r}', f'loader={self.loader!r}']
        if self.origin is not None:
            args.append(f'origin={self.origin!r}')
        if self.submodule_search_locations is not None:
            args.append(f'submodule_search_locations={self.submodule_search_locations}')
        return f'{self.__class__.__name__}({", ".join(args)})'

    def __eq__(self, other):
        smsl = self.submodule_search_locations
        try:
            return (self.name == other.name and
                    self.loader == other.loader and
                    self.origin == other.origin and
                    smsl == other.submodule_search_locations and
                    self.cached == other.cached and
                    self.has_location == other.has_location)
        except AttributeError:
            return NotImplemented

    @property
    def cached(self):
        # This interpreter has no bytecode cache (sys.implementation.cache_tag is None).
        return self._cached

    @cached.setter
    def cached(self, cached):
        self._cached = cached

    @property
    def parent(self):
        """The name of the module's parent."""
        if self.submodule_search_locations is None:
            return self.name.rpartition('.')[0]
        else:
            return self.name

    @property
    def has_location(self):
        return self._set_fileattr

    @has_location.setter
    def has_location(self, value):
        self._set_fileattr = bool(value)


class SourceLoader:
    def __init__(self, name, filename):
        self.name = name
        self.path = filename

    def create_module(self, spec):
        return None

    def exec_module(self, module):
        source = self.get_source(self.name)
        exec(compile(source, self.path, 'exec'), module.__dict__)

    def get_filename(self, fullname):
        if fullname != self.name:
            raise ImportError('loader for ' + self.name + ' cannot handle ' + fullname)
        return self.path

    def get_code(self, fullname):
        import sys
        self.get_filename(fullname)
        if fullname in sys.builtin_module_names:
            return None
        return compile(self.get_source(fullname), self.path, 'exec')

    def get_source(self, fullname):
        self.get_filename(fullname)
        return _load_module(fullname, 'source')

    # Expose filesystem resources through the source loader protocol.
    def get_resource_reader(self, fullname):
        self.get_filename(fullname)
        # Native namespace locations encode all search roots after a NUL marker.
        if self.path.startswith("\0"):
            from importlib._bootstrap_external import NamespaceLoader, PathFinder
            loader = NamespaceLoader(fullname, self.path[1:].split("\0"), PathFinder._get_spec)
            return loader.get_resource_reader(fullname)
        from importlib.readers import FileReader
        return FileReader(self)

    def is_package(self, fullname):
        return self.get_filename(fullname).endswith('/__init__.py')

class SourceFinder:
    _native_source_finder = True
    def find_spec(self, fullname, path=None, target=None):
        filename = _load_module(fullname, True)
        if filename is None:
            return None
        return make_spec(fullname, filename)

    def invalidate_caches(self):
        pass

# Library modules reported as frozen: name -> (is a package, original name).
FROZEN_MODULES = {
    '__hello__': (False, '__hello__'),
    '__phello__': (True, '__phello__'),
    '__phello__.spam': (False, '__phello__.spam'),
    '__phello__.ham': (True, '__phello__.ham'),
    '__phello__.ham.eggs': (False, '__phello__.ham.eggs'),
}
# 0 follows the default (frozen modules in use), 1 forces them on, -1 off.
frozen_override = 0

class FrozenFinder:
    """Meta path entry that hands frozen names to the frozen importer."""
    @classmethod
    def find_spec(cls, fullname, path=None, target=None):
        if frozen_override < 0 or fullname not in FROZEN_MODULES:
            return None
        from importlib._bootstrap import FrozenImporter
        return FrozenImporter.find_spec(fullname, path, target)

def make_spec(name, filename):
    import sys as _system
    bootstrap = _system.modules.get('importlib._bootstrap')
    factory = getattr(bootstrap, 'ModuleSpec', ModuleSpec) if bootstrap is not None else ModuleSpec
    loader = SourceLoader(name, filename)
    spec = factory(name, loader, origin=filename, is_package=filename.endswith('/__init__.py'))
    spec.has_location = True
    if name in getattr(_system, 'builtin_module_names', ()):
        spec.origin = 'built-in'
        spec.has_location = False
    if spec.submodule_search_locations is not None:
        spec.submodule_search_locations.append(filename.rsplit('/', 1)[0])
    return spec

def find_custom(name, parent_path):
    import sys
    for finder in sys.meta_path:
        find_spec = getattr(finder, 'find_spec', None)
        if find_spec is None:
            continue
        spec = find_spec(name, parent_path, None)
        if spec is not None:
            if type(finder) is SourceFinder and type(spec.loader) is SourceLoader:
                return True, None
            from importlib._bootstrap import _load_unlocked
            return False, _load_unlocked(spec)
    raise ModuleNotFoundError("No module named '" + name + "'")
