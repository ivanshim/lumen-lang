# Register filesystem discovery only when installed importers do not provide it.
import sys
import _imp
from importlib.machinery import PathFinder as _distribution_path_finder

_native_distribution_finder = getattr(_imp, '_NativeFinder', None)
_distribution_providers = (_distribution_path_finder, MetadataPathFinder)
if isinstance(_native_distribution_finder, type) and hasattr(_native_distribution_finder, 'find_distributions'):
    _distribution_providers += (_native_distribution_finder,)
if not any(isinstance(finder, type) and issubclass(finder, _distribution_providers)
           for finder in sys.meta_path):
    sys.meta_path.append(MetadataPathFinder)
del _native_distribution_finder, _distribution_providers, _distribution_path_finder
