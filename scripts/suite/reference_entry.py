#!/usr/bin/env python3
"""Import an unmodified reference module through unittest's normal loader."""
import os as _os
import sys as _sys
import unittest as _unittest
import test as _test

def _main():
    path = _sys.argv[1]
    package_entry = _os.path.basename(path) == '__main__.py'
    directory = _os.path.dirname(path)
    suite = _os.path.dirname(directory) if package_entry else directory
    _test.__path__.insert(0, suite)
    name = 'test.' + (_os.path.basename(directory) if package_entry else _os.path.basename(path)[:-3])
    module = __import__(name, fromlist=['*'])
    _unittest.main(module=module, argv=[path])

_main()
