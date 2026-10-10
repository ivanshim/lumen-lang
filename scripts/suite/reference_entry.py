"""Run a reference suite through its genuinely imported module."""

def _run():
    import sys
    import importlib
    import test
    import unittest

    directory, name, filename = sys.argv[1:4]
    sys.argv = [filename]
    test.__path__.insert(0, directory)
    module = importlib.import_module(name)
    unittest.main(module=module, argv=sys.argv)


if __name__ == "__main__":
    _run()
