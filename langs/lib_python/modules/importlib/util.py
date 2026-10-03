# Runtime adapter derived from CPython v3.14.8 / 8e6e75d9102e,
# Lib/importlib/_bootstrap_external.py (cache_from_source); PSF License.
# Where a bytecode cache would stand beside a source file, named the
# way the reference names it. This runtime reads source as written and
# keeps no bytecode; where the implementation names no cache tag the
# reference's own refusal is given rather than a made-up name.
import os as _os
import sys
import warnings as _warnings

_PYCACHE = '__pycache__'
_OPT = 'opt-'
BYTECODE_SUFFIXES = ['.pyc']


# POSIX path helpers from the pinned source above. These deliberately
# differ from os.path: split at the last separator, strip component
# endings when joining, and remove one leading ./ from a relative head.
path_sep = '/'
path_separators = '/'


def _path_join(*path_parts):
    """Replacement for os.path.join()."""
    return path_sep.join([part.rstrip(path_separators)
                          for part in path_parts if part])


def _path_split(path):
    """Replacement for os.path.split()."""
    i = max(path.rfind(p) for p in path_separators)
    if i < 0:
        return '', path
    return path[:i], path[i + 1:]


def _path_isabs(path):
    """Replacement for os.path.isabs."""
    return path.startswith(path_separators)


def _path_abspath(path):
    """Replacement for os.path.abspath."""
    if not _path_isabs(path):
        for sep in path_separators:
            path = path.removeprefix(f".{sep}")
        return _path_join(_os.getcwd(), path)
    else:
        return path


def cache_from_source(path, debug_override=None, *, optimization=None):
    if debug_override is not None:
        _warnings.warn('the debug_override parameter is deprecated; use '
                       "'optimization' instead", DeprecationWarning)
        if optimization is not None:
            raise TypeError('debug_override or optimization must be set to None')
        optimization = '' if debug_override else 1
    path = _os.fspath(path)
    head, tail = _path_split(path)
    base, sep, rest = tail.rpartition('.')
    tag = sys.implementation.cache_tag
    if tag is None:
        raise NotImplementedError('sys.implementation.cache_tag is None')
    almost_filename = ''.join([(base if base else rest), sep, tag])
    if optimization is None:
        if sys.flags.optimize == 0:
            optimization = ''
        else:
            optimization = sys.flags.optimize
    optimization = str(optimization)
    if optimization != '':
        if not optimization.isalnum():
            raise ValueError(f'{optimization!r} is not alphanumeric')
        almost_filename = f'{almost_filename}.{_OPT}{optimization}'
    filename = almost_filename + BYTECODE_SUFFIXES[0]
    prefix = getattr(sys, 'pycache_prefix', None)
    if prefix is not None:
        head = _path_abspath(head)
        if head[1:2] == ':' and head[0:1] not in path_separators:
            head = head[2:]
        return _path_join(prefix, head.lstrip(path_separators), filename)
    return _path_join(head, _PYCACHE, filename)
