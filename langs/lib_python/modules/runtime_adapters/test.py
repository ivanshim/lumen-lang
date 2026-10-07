# Locate reference tests and support packages beside the embedded test library.
import os as _archive_os

_archive_root = _archive_os.path.dirname(_archive_os.path.dirname(
    _archive_os.path.dirname(_archive_os.path.dirname(
        _archive_os.path.dirname(__file__)))))
_archive_tests = _archive_os.path.join(_archive_root, 'tests', 'python-3.14.8')
for _archive_path in (_archive_tests, _archive_os.path.join(_archive_tests, 'test')):
    if _archive_path not in __path__:
        __path__.append(_archive_path)
