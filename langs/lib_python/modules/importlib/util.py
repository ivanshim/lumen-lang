# Runtime adapter derived from CPython v3.14.8 / 8e6e75d9102e,
# Lib/importlib/_bootstrap_external.py (cache_from_source); PSF License.
# Where a bytecode cache would be kept beside a source file, named the
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
    if isinstance(path, bytes):
        head, tail = path.rsplit(b'/', 1) if b'/' in path else (b'', path)
        base, sep, rest = tail.rpartition(b'.')
        apart = b'/'
    else:
        head, tail = path.rsplit('/', 1) if '/' in path else ('', path)
        base, sep, rest = tail.rpartition('.')
        apart = '/'
    tag = sys.implementation.cache_tag
    if tag is None:
        raise NotImplementedError('sys.implementation.cache_tag is None')
    if isinstance(path, bytes):
        almost_filename = b''.join([(base if base else rest), sep, tag.encode()])
    else:
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
    if getattr(sys, 'pycache_prefix', None) is not None:
        raise NotImplementedError('sys.pycache_prefix is not supported here')
    if isinstance(path, bytes):
        return head + b'/' + _PYCACHE.encode() + b'/' + filename
    return head + '/' + _PYCACHE + '/' + filename
