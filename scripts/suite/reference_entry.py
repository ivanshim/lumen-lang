#!/usr/bin/env python3
"""Run an unmodified reference module through unittest's normal loader."""
import importlib
import os
import sys
import types
import unittest

path = sys.argv[1]
import test
package_entry = os.path.basename(path) == '__main__.py'
suite = os.path.dirname(os.path.dirname(path)) if package_entry else os.path.dirname(path)
if suite not in test.__path__:
    test.__path__.insert(0, suite)
if package_entry:
    package_name = 'test.' + os.path.basename(os.path.dirname(path))
    package = importlib.import_module(package_name)
    directory = os.path.dirname(path)
    if directory not in package.__path__:
        package.__path__.insert(0, directory)
    # Let the package's canonical load_tests hook discover its real modules.
    unittest.main(module=package, argv=[path] + sys.argv[2:])
    sys.exit(0)
module_name = 'test.' + path.rsplit('/', 1)[-1][:-3]
module = types.ModuleType(module_name)
module.__file__ = path
module.__package__ = 'test'
sys.modules[module_name] = module
with open(path, encoding='utf-8') as source:
    exec(compile(source.read(), path, 'exec'), module.__dict__)
unittest.main(module=module, argv=[path] + sys.argv[2:])
