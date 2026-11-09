# Runtime bootstrap adapter for CPython v3.14.8 Lib/importlib/_bootstrap.py; PSF License.
# Initialize importlib with the available warning backend.
def _setup(sys_module, _imp_module):
    """Setup importlib by importing needed built-in modules and injecting them
    into the global namespace.

    As sys is needed for sys.modules access and _imp is needed to load built-in
    modules, those two modules must be explicitly passed in.

    """
    global _imp, sys, _blocking_on, _warnings
    _imp = _imp_module
    sys = sys_module

    # Set up the spec for existing builtin/frozen modules.
    module_type = type(sys)
    for name, module in sys.modules.items():
        if isinstance(module, module_type):
            if name in sys.builtin_module_names:
                loader = BuiltinImporter
            elif _imp.is_frozen(name):
                loader = FrozenImporter
            else:
                continue
            spec = _spec_from_module(module, loader)
            _init_module_attrs(spec, module)
            if loader is FrozenImporter:
                loader._fix_up_module(module)

    # Directly load built-in modules needed during bootstrap.
    self_module = sys.modules[__name__]
    for builtin_name in ('_thread', '_weakref'):
        if builtin_name not in sys.modules:
            builtin_module = _builtin_from_name(builtin_name)
        else:
            builtin_module = sys.modules[builtin_name]
        setattr(self_module, builtin_name, builtin_module)

    # Bootstrap can also run when an optional warning extension is blocked.
    try:
        _warnings = __load_module('_warnings')
    except ImportError:
        _warnings = __load_module('warnings')

    # Instantiation requires _weakref to have been set.
    _blocking_on = _WeakValueDictionary()
    if _imp._NativeFinder not in sys.meta_path:
        sys.meta_path.append(_imp._NativeFinder)
    # The reference's own importers for builtin and frozen modules.
    for importer in (BuiltinImporter, FrozenImporter):
        if importer not in sys.meta_path:
            sys.meta_path.append(importer)



# The reference defines the importers in its frozen module _frozen_importlib,
# and what is defined there carries that module's name.
for _name, _value in list(globals().items()):
    if getattr(_value, '__module__', None) == __name__ and isinstance(_value, (type, type(_setup))):
        _value.__module__ = '_frozen_importlib'
del _name, _value
