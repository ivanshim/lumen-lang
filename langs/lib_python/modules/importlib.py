# Bringing a module in is the reader's own work. This module names the
# parts of that machinery a program is allowed to ask after, and the
# one call that settles any finder state. The reader reads directories
# as it goes and keeps nothing of their listing between imports, so
# there is never a stale cache to clear and the ask is answered with
# nothing to do. The utility part keeps the reference's own naming of
# where a bytecode cache would stand beside a source file.
import sys
import warnings as _warnings


def invalidate_caches():
    return None


class _Util:
    _PYCACHE = '__pycache__'
    _OPT = 'opt-'
    BYTECODE_SUFFIXES = ['.pyc']

    # The naming is CPython v3.14.8 / 8e6e75d9102e,
    # Lib/importlib/_bootstrap_external.py's cache_from_source; PSF License.
    def cache_from_source(self, path, debug_override=None, *, optimization=None):
        if debug_override is not None:
            _warnings.warn('the debug_override parameter is deprecated; use '
                           "'optimization' instead", DeprecationWarning)
            if optimization is not None:
                raise TypeError('debug_override or optimization must be set to None')
            optimization = '' if debug_override else 1
        if not isinstance(path, str):
            raise TypeError('expected str, bytes or os.PathLike object, not ' + str(type(path)))
        head, tail = path.rsplit('/', 1) if '/' in path else ('', path)
        base, sep, rest = tail.rpartition('.')
        # The tag the reference writes caches under; this runtime answers
        # the same language version, so the names agree with it.
        info = sys.version_info
        tag = 'cpython-%d%d' % (info[0], info[1])
        almost_filename = ''.join([(base if base else rest), sep, tag])
        if optimization is None:
            optimization = ''
        optimization = str(optimization)
        if optimization != '':
            if not optimization.isalnum():
                raise ValueError(f'{optimization!r} is not alphanumeric')
            almost_filename = f'{almost_filename}.{self._OPT}{optimization}'
        filename = almost_filename + self.BYTECODE_SUFFIXES[0]
        if head:
            return head + '/' + self._PYCACHE + '/' + filename
        return self._PYCACHE + '/' + filename


util = _Util()
