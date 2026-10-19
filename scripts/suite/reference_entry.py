"""Run a reference suite through its genuinely imported module."""

def _run():
    import sys
    import importlib
    import test
    import unittest

    if len(sys.argv) == 2:
        # The on-box checker passes a filename; staged_reference supplies
        # the directory and canonical module name as separate arguments.
        filename = sys.argv[1]
        directory, _, leaf = filename.rpartition('/')
        if leaf == '__main__.py':
            directory, _, leaf = directory.rpartition('/')
            name = 'test.' + leaf
        else:
            name = 'test.' + leaf[:-3]
    else:
        directory, name, filename = sys.argv[1:4]
    sys.argv = [filename]
    test.__path__.insert(0, directory)
    module = importlib.import_module(name)
    unittest.main(module=module, argv=sys.argv)


if __name__ == "__main__":
    _run()
