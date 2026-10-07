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


def _search_directories():
    # The directories a module is looked for in, in order: the current
    # directory first (where -m puts its own first path entry), then the
    # module search path, each entry made absolute and kept once. A name
    # that names the current directory over again is not searched twice.
    seen = []
    for entry in [os.getcwd()] + list(sys.path):
        if not entry:
            continue
        entry = entry if os.path.isabs(entry) else os.path.normpath(os.path.join(os.getcwd(), entry))
        if entry not in seen:
            seen.append(entry)
    return seen


def _get_module_details(mod_name):
    # A dotted name resolved the way -m resolves it: the current
    # directory first, then each directory on the module search path. A
    # name ending in .__main__ names that submodule outright; any other
    # name names a module file, or a package run through the __main__.py
    # its directory carries.
    if mod_name.endswith(".__main__"):
        pkg = mod_name[: -len(".__main__")]
        rel = pkg.replace(".", "/") + "/__main__.py"
        for directory in _search_directories():
            candidate = os.path.join(directory, rel)
            if os.path.isfile(candidate):
                return mod_name, candidate, pkg
        raise ImportError("No module named " + mod_name)
    rel = mod_name.replace(".", "/")
    for directory in _search_directories():
        candidate = os.path.join(directory, rel + ".py")
        if os.path.isfile(candidate):
            return mod_name, candidate, mod_name.rpartition(".")[0]
    for directory in _search_directories():
        candidate = os.path.join(directory, rel, "__main__.py")
        if os.path.isfile(candidate):
            return mod_name + ".__main__", candidate, mod_name
    raise ImportError("No module named " + mod_name)


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
