"""Locate and run Python modules without importing them first.

Only the piece test_argparse needs is carried: _run_module_as_main runs
a module's file as __main__, naming it the module so argparse reads the
module's name and origin, and leaving sys.argv alone when asked to.
"""

import os
import sys


class _Spec:
    def __init__(self, name, origin, parent):
        self.name = name
        self.origin = origin
        self.parent = parent


# Normalize sys.path entries without changing their search order.
def _search_directories():
    seen = []
    for entry in sys.path:
        entry = os.path.abspath(entry or os.curdir)
        if entry not in seen:
            seen.append(entry)
    return seen


# Resolve one module component, initializing regular packages when selected.
def _find_component(name, directories):
    leaf = name.rpartition(".")[2]
    namespaces = []
    for directory in directories:
        package_dir = os.path.join(directory, leaf)
        initializer = os.path.join(package_dir, "__init__.py")
        if os.path.isfile(initializer):
            package = __import__(name, fromlist=["__path__"])
            return None, list(package.__path__)
        candidate = os.path.join(directory, leaf + ".py")
        if os.path.isfile(candidate):
            return candidate, None
        if os.path.isdir(package_dir):
            namespaces.append(package_dir)
    if namespaces:
        return None, namespaces
    raise ImportError("No module named " + name)


# Resolve dotted names through each selected package's initialized path.
def _get_module_details(mod_name):
    directories = _search_directories()
    parts = mod_name.split(".")
    name = ""
    for part in parts:
        name = name + "." + part if name else part
        loaded = sys.modules.get(name)
        if loaded is not None and hasattr(loaded, "__path__"):
            origin, directories = None, list(loaded.__path__)
        else:
            origin, package_path = _find_component(name, directories)
            if package_path is not None:
                directories = package_path
        if origin is not None:
            if name != mod_name:
                raise ImportError("No module named " + mod_name + "; " + name + " is not a package")
            return name, origin, name.rpartition(".")[0]
    if mod_name.endswith(".__main__"):
        raise ImportError("Cannot use package as __main__ module")
    main_name = mod_name + ".__main__"
    origin, package_path = _find_component(main_name, directories)
    if origin is None:
        raise ImportError("Cannot use package as __main__ module")
    return main_name, origin, mod_name


def _run_module_as_main(mod_name, alter_argv=True):
    """Run the module named mod_name as the __main__ module.

    The module is reached the way -m reaches it: the dotted name as a
    path, a package run through its __main__.py. alter_argv=False keeps
    sys.argv[0] as the runner that called here, which is what argparse
    reads when a program runs another module without renaming itself.
    """
    name, origin, parent = _get_module_details(mod_name)
    with open(origin, encoding="utf-8") as f:
        source = f.read()
    main = sys.modules["__main__"]
    # The module runs under the name __main__, as any __main__ does; its
    # spec still carries the module's own name for argparse to read.
    main.__spec__ = _Spec(name, origin, parent)
    main.__file__ = origin
    main.__package__ = parent
    if alter_argv:
        sys.argv[0] = origin
    code = compile(source, origin, "exec")
    exec(code, main.__dict__)
