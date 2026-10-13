#!/usr/bin/env python3
"""Run an unmodified reference module through unittest's normal loader."""
import importlib
import os
import sys
import test
import unittest

path = os.path.abspath(sys.argv[1])
module_name = 'test.' + os.path.basename(path)[:-3]
test.__path__.insert(0, os.path.dirname(path))
module = importlib.import_module(module_name)
unittest.main(module=module, argv=[path])
