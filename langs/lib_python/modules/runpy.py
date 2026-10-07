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


def _find_module(mod_name):
    path = mod_name.replace(".", "/")
    for candidate in (path + ".py", path + "/__main__.py"):
        if os.path.exists(candidate):
            return candidate
    raise ImportError("No module named " + mod_name)


def _run_module_as_main(mod_name, alter_argv=True):
    """Run the module named mod_name as the __main__ module.

    The module is reached the way -m reaches it: the dotted name as a
    path, a package run through its __main__.py. alter_argv=False keeps
    sys.argv[0] as the runner that called here, which is what argparse
    reads when a program runs another module without renaming itself.
    """
    candidate = _find_module(mod_name)
    with open(candidate, encoding="utf-8") as f:
        source = f.read()
    main = sys.modules["__main__"]
    name = mod_name + (".__main__" if candidate.endswith("/__main__.py") else "")
    parent = mod_name.rsplit(".", 1)[0] if "." in mod_name else ""
    main.__spec__ = _Spec(name, candidate, parent)
    if alter_argv:
        sys.argv[0] = candidate
    code = compile(source, candidate, "exec")
    exec(code, main.__dict__)
