# Route non-file loaders through upstream module creation and initialization.
import sys
from importlib import _bootstrap, _bootstrap_external

# Find a non-native spec while bootstrap imports stay on the native road.
def find(fullname, path):
    for finder in list(sys.meta_path):
        try:
            find_spec = finder.find_spec
        except AttributeError:
            continue
        spec = find_spec(fullname, path)
        if spec is None:
            continue
        if getattr(finder, "_native_source_finder", False):
            return None
        if type(spec.loader) is _bootstrap_external.SourceFileLoader:
            import os
            native_file = __load_module(fullname, True)
            if native_file is not None:
                native_file = os.path.abspath(native_file)
                source_file = os.path.abspath(spec.origin)
                # Some embedded packages retain a flat source file and a virtual initializer.
                virtual_package = native_file.endswith('/__init__.py') and not os.path.isfile(native_file)
                library_root = os.path.dirname(os.path.dirname(_bootstrap_external.__file__))
                embedded = source_file.startswith(library_root + os.sep)
                if embedded and (source_file == native_file or virtual_package and source_file == native_file[:-12] + '.py'):
                    return None
        return spec
    return None

# Initialize outside the finder guard so archive modules can import one another.
def load(spec):
    module = _bootstrap._load_unlocked(spec)
    parent, separator, child = spec.name.rpartition('.')
    if separator:
        bind(parent, child, module)
    return module


def bind(parent, child, module):
    try:
        setattr(sys.modules[parent], child, module)
    except AttributeError:
        import warnings
        warnings.warn(f"Cannot set an attribute on {parent!r} for child module {child!r}", ImportWarning)
