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


def cache_from_source(path, debug_override=None, *, optimization=None):
    if debug_override is not None:
        _warnings.warn('the debug_override parameter is deprecated; use '
                       "'optimization' instead", DeprecationWarning)
        if optimization is not None:
            raise TypeError('debug_override or optimization must be set to None')
        optimization = '' if debug_override else 1
    path = _os.fspath(path)
    head, tail = _os.path.split(path)
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
        head = _os.path.abspath(head)
        if head[1:2] == ':' and head[0:1] not in '/':
            head = head[2:]
        return _os.path.join(prefix, head.lstrip('/'), filename)
    return _os.path.join(head, _PYCACHE, filename)
