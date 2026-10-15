#!/usr/bin/env python3
"""Import an unmodified reference module through unittest's normal loader."""
import os
import sys
import unittest
import test

path = sys.argv[1]
package_entry = os.path.basename(path) == '__main__.py'
directory = os.path.dirname(path)
suite = os.path.dirname(directory) if package_entry else directory
test.__path__.insert(0, suite)
name = 'test.' + (os.path.basename(directory) if package_entry else os.path.basename(path)[:-3])
module = __import__(name, fromlist=['*'])
unittest.main(module=module, argv=[path])
