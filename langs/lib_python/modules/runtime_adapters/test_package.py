# Reference suites keep their test modules beside the support package.
import os as _os
import sys as _sys

_candidates = list(_sys.path)
_parent = _os.path.dirname(_os.path.abspath(_sys.argv[0]))
while True:
    _candidates.append(_parent)
    _next = _os.path.dirname(_parent)
    if _next == _parent:
        break
    _parent = _next
_suite_version = 'python-' + '.'.join(str(_part) for _part in _sys.version_info[:3])
for _base in list(_candidates):
    _archive = _os.path.join(_base, 'tests', _suite_version)
    if _os.path.isdir(_archive):
        _candidates.append(_archive)
for _directory in _candidates:
    _directory = _os.path.abspath(_directory)
    if (_os.path.isfile(_os.path.join(_directory, 'test', '__init__.py'))
            and _os.path.isdir(_os.path.join(_directory, 'test', 'support'))
            and _directory not in __path__):
        __path__.append(_directory)
