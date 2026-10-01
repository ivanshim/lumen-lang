# Portions of CPython 3b564385e4c9 Lib/importlib/_bootstrap_external.py; PSF License.
# Source loading uses the available filesystem and compiler. Native extensions
# and CPython bytecode cannot execute in these kernels.
import os as _os
from . import _bootstrap
SOURCE_SUFFIXES = ['.py']
BYTECODE_SUFFIXES = ['.pyc']
EXTENSION_SUFFIXES = []
MAGIC_NUMBER = (3705).to_bytes(2, 'little') + b'\r\n'

def _path_abspath(path):
    return _os.path.abspath(path)

def _path_split(path):
    return _os.path.split(path)

def _get_supported_file_loaders():
    return [(SourceFileLoader, SOURCE_SUFFIXES), (SourcelessFileLoader, BYTECODE_SUFFIXES)]

_POPULATE = object()


def spec_from_file_location(name, location=None, *, loader=None,
                            submodule_search_locations=_POPULATE):
    """Return a module spec based on a file location.

    To indicate that the module is a package, set
    submodule_search_locations to a list of directory paths.  An
    empty list is sufficient, though its not otherwise useful to the
    import system.

    The loader must take a spec as its only __init__() arg.

    """
    if location is None:
        # The caller may simply want a partially populated location-
        # oriented spec.  So we set the location to a bogus value and
        # fill in as much as we can.
        location = '<unknown>'
        if hasattr(loader, 'get_filename'):
            # ExecutionLoader
            try:
                location = loader.get_filename(name)
            except ImportError:
                pass
    else:
        location = _os.fspath(location)
        try:
            location = _path_abspath(location)
        except OSError:
            pass

    # If the location is on the filesystem, but doesn't actually exist,
    # we could return None here, indicating that the location is not
    # valid.  However, we don't have a good way of testing since an
    # indirect location (e.g. a zip file or URL) will look like a
    # non-existent file relative to the filesystem.

    spec = _bootstrap.ModuleSpec(name, loader, origin=location)
    spec._set_fileattr = True

    # Pick a loader if one wasn't provided.
    if loader is None:
        for loader_class, suffixes in _get_supported_file_loaders():
            if location.endswith(tuple(suffixes)):
                loader = loader_class(name, location)
                spec.loader = loader
                break
        else:
            return None

    # Set submodule_search_paths appropriately.
    if submodule_search_locations is _POPULATE:
        # Check the loader.
        if hasattr(loader, 'is_package'):
            try:
                is_package = loader.is_package(name)
            except ImportError:
                pass
            else:
                if is_package:
                    spec.submodule_search_locations = []
    else:
        spec.submodule_search_locations = submodule_search_locations
    if spec.submodule_search_locations == []:
        if location:
            dirname = _path_split(location)[0]
            spec.submodule_search_locations.append(dirname)

    return spec



class SourceFileLoader:
    def __init__(self, fullname, path):
        self.name = fullname
        self.path = path
    def get_filename(self, fullname):
        if fullname != self.name:
            raise ImportError('loader cannot handle ' + fullname)
        return self.path
    def is_package(self, fullname):
        return _os.path.basename(self.get_filename(fullname)) == '__init__.py'
    def create_module(self, spec):
        return None
    def get_data(self, path):
        with open(path, 'rb') as file:
            return file.read()
    def get_source(self, fullname):
        import tokenize
        with tokenize.open(self.get_filename(fullname)) as file:
            return file.read()
    def get_code(self, fullname):
        return compile(self.get_source(fullname), self.path, 'exec')
    def exec_module(self, module):
        exec(self.get_code(module.__name__), module.__dict__)

class SourcelessFileLoader(SourceFileLoader):
    def get_code(self, fullname):
        raise ImportError('CPython bytecode is not supported', name=fullname, path=self.path)

class ExtensionFileLoader(SourceFileLoader):
    def get_code(self, fullname):
        raise ImportError('native Python extensions are not supported', name=fullname, path=self.path)

class FileFinder:
    def __init__(self, path, *loader_details):
        self.path = path
        self._loaders = loader_details or _get_supported_file_loaders()
    def invalidate_caches(self):
        pass
    def find_spec(self, fullname, target=None):
        tail = fullname.rsplit('.', 1)[-1]
        for loader, suffixes in self._loaders:
            for suffix in suffixes:
                for path in (_os.path.join(self.path, tail, '__init__' + suffix), _os.path.join(self.path, tail + suffix)):
                    if _os.path.isfile(path):
                        return spec_from_file_location(fullname, path, loader=loader(fullname, path))
        return None

class PathFinder:
    @classmethod
    def find_spec(cls, fullname, path=None, target=None):
        import sys
        for directory in (sys.path if path is None else path):
            spec = FileFinder(directory).find_spec(fullname, target)
            if spec is not None:
                return spec
        return None

def _get_cached(filename):
    if filename.endswith('.pyc'):
        return filename
    return None
