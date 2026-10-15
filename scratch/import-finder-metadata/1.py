import sys
import importlib.util

class Loader:
    def create_module(self, spec):
        return None
    def exec_module(self, module):
        module.loaded = True

class Finder:
    def find_spec(self, fullname, path=None, target=None):
        if fullname not in ('finder_pkg', 'finder_pkg.child'):
            return None
        package = fullname == 'finder_pkg'
        spec = importlib.util.spec_from_loader(fullname, Loader(), is_package=package)
        spec.origin = '/finder/pkg/__init__.py' if package else '/finder/pkg/child.py'
        spec.has_location = True
        if package:
            spec.submodule_search_locations.append('/finder/pkg')
        return spec

sys.meta_path.insert(0, Finder())
import finder_pkg.child
assert finder_pkg.__file__ == '/finder/pkg/__init__.py'
assert finder_pkg.__package__ == 'finder_pkg'
assert finder_pkg.__path__ == ['/finder/pkg']
assert finder_pkg.child is sys.modules['finder_pkg.child']
assert finder_pkg.child.__file__ == '/finder/pkg/child.py'
assert finder_pkg.child.__package__ == 'finder_pkg'
assert finder_pkg.child.loaded
print('finder metadata and parent binding pass')
