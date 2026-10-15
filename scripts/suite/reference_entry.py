#!/usr/bin/env python3
"""Run an unmodified reference module through unittest's normal loader."""
import sys
import types
import unittest

path = sys.argv[1]
module_name = 'test.' + path.rsplit('/', 1)[-1][:-3]
module = types.ModuleType(module_name)
module.__file__ = path
module.__package__ = 'test'
sys.modules[module_name] = module
with open(path, encoding='utf-8') as source:
    exec(compile(source.read(), path, 'exec'), module.__dict__)
unittest.main(module=module, argv=[path])
