# Imports use the native reader; cache invalidation reaches registered finders.


def invalidate_caches():
    import sys
    for finder in list(sys.meta_path):
        if hasattr(finder, "invalidate_caches"):
            finder.invalidate_caches()

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

# The reader answers `import` itself; the reference machinery a library
# asks after is installed here beside it, once, so `path_hooks`,
# `meta_path` and `path_importer_cache` hold the finders CPython's own
# FileFinder, PathFinder and zipimporter provide.


def _zip_path_hook(path):
    # Ask zipimport only when a path entry is offered: importing it
    # while importlib is being read would close a circle, so the hook
    # stands in its place and brings it in on the first ask.
    import zipimport
    return zipimport.zipimporter(path)

import sys as _machinery_sys

if not getattr(_machinery_sys, '_lumen_importers_ready', False):
    try:
        import importlib._bootstrap as _machinery_bootstrap
        import importlib._bootstrap_external as _machinery_external
    except ImportError:
        pass
    else:
        _machinery_external._install(_machinery_bootstrap)
        # Replace the lazy bootstrap proxy at its original search priority.
        proxy = getattr(_machinery_sys, '_PathFinder', None)
        if proxy in _machinery_sys.meta_path:
            position = _machinery_sys.meta_path.index(proxy)
            _machinery_sys.meta_path.remove(_machinery_external.PathFinder)
            _machinery_sys.meta_path[position] = _machinery_external.PathFinder
        for name in ('_zip_path_hook', '_file_path_hook'):
            hook = getattr(_machinery_sys, name, None)
            if hook in _machinery_sys.path_hooks:
                _machinery_sys.path_hooks.remove(hook)
        # Metadata has no caches before its optional module is loaded.
        def _invalidate_path_caches():
            for path, finder in list(_machinery_sys.path_importer_cache.items()):
                if finder is None or not _machinery_external._path_isabs(path):
                    del _machinery_sys.path_importer_cache[path]
                elif hasattr(finder, 'invalidate_caches'):
                    finder.invalidate_caches()
            _machinery_external._NamespacePath._epoch += 1
            metadata = _machinery_sys.modules.get('importlib.metadata')
            if metadata is not None:
                metadata.MetadataPathFinder.invalidate_caches()
        _machinery_external.PathFinder.invalidate_caches = staticmethod(_invalidate_path_caches)
        if _zip_path_hook not in _machinery_sys.path_hooks:
            _machinery_sys.path_hooks.insert(0, _zip_path_hook)
        _machinery_sys._lumen_importers_ready = True


def _attach_native_spec(module):
    # The reader binds a module's own file and package path, but not
    # the spec the reference keeps beside them. This builds that spec
    # from the bindings, so a caller asking after the module's loader
    # is answered the way the reference answers.
    import importlib._bootstrap as _bootstrap
    import importlib._bootstrap_external as _bootstrap_external
    name = module.__name__
    origin = getattr(module, '__file__', None)
    loader = _bootstrap_external.SourceFileLoader(name, origin) if origin is not None else None
    spec = _bootstrap.ModuleSpec(name, loader, origin=origin)
    if origin is not None:
        # A spec whose origin is a file answers `has_location`, as one
        # built by spec_from_file_location does, so the module's repr
        # stays "<module 'name' from 'file'>".
        spec.has_location = True
    search = getattr(module, '__path__', None)
    if search is not None:
        spec.submodule_search_locations = list(search)
    module.__spec__ = spec
    return module
