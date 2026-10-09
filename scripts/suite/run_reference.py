"""Import the requested source before unittest discovery."""
import importlib
import os
import sys
import unittest

name, filename = sys.argv[1:3]
parent, _, child = name.rpartition(".")
search = importlib.import_module(parent).__path__ if parent else sys.path
search.insert(0, os.path.dirname(os.path.abspath(filename)))
sys.modules.pop(name, None)
module = importlib.import_module(name)
if os.path.realpath(module.__file__) != os.path.realpath(filename):
    raise ImportError("requested source was not loaded: " + filename)
unittest.main(module=module, argv=[filename, *sys.argv[3:]])
