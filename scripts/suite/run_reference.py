"""Load the requested source under its import name before unittest discovery."""
import importlib.util
import sys
import unittest

name, filename = sys.argv[1:3]
parent, _, child = name.rpartition(".")
package = importlib.import_module(parent) if parent else None
spec = importlib.util.spec_from_file_location(name, filename)
module = importlib.util.module_from_spec(spec)
sys.modules[name] = module
spec.loader.exec_module(module)
if package is not None:
    setattr(package, child, module)
unittest.main(module=module, argv=[filename, *sys.argv[3:]])
