# Installation paths for the embedded Python library.
import os
import sys

def get_paths(scheme=None, vars=None, expand=True):
    root = os.path.dirname(__file__)
    paths = dict(stdlib=root, platstdlib=root, purelib=root, platlib=root,
                 include=os.path.join(root, 'include'), platinclude=os.path.join(root, 'include'),
                 scripts=os.path.dirname(sys.executable), data=root)
    if vars:
        paths.update(vars)
    return paths

def get_path(name, scheme=None, vars=None, expand=True):
    return get_paths(scheme, vars, expand)[name]

def is_python_build(check_home=False):
    return False
