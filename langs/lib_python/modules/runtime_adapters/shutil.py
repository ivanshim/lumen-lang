try:
    import posix as _shutil_posix
except ImportError:
    _shutil_posix = None

if _shutil_posix is not None and hasattr(_shutil_posix, '_rmtree') and _use_fd_functions:
    _shutil_orig_rmtree = rmtree
    _SHUTIL_ORIG_IMPL = _rmtree_impl
    _SHUTIL_ORIG_SAFE_FD = _rmtree_safe_fd
    _SHUTIL_ORIG_UNSAFE = _rmtree_unsafe
    _SHUTIL_ORIG_LSTAT = os.lstat
    _SHUTIL_ORIG_OPEN = os.open
    _SHUTIL_ORIG_FSTAT = os.fstat
    _SHUTIL_ORIG_SAMESTAT = os.path.samestat
    _SHUTIL_ORIG_SCANDIR = os.scandir
    _SHUTIL_ORIG_UNLINK = os.unlink
    _SHUTIL_ORIG_RMDIR = os.rmdir
    _SHUTIL_ORIG_CLOSE = os.close
    _SHUTIL_ORIG_FSPATH = os.fspath

    def _shutil_fast_rmtree(path, ignore_errors=False, onerror=None, *, onexc=None, dir_fd=None):
        if (onerror is not None or onexc is not None or dir_fd is not None
                or _rmtree_impl is not _SHUTIL_ORIG_IMPL
                or _rmtree_safe_fd is not _SHUTIL_ORIG_SAFE_FD
                or _rmtree_unsafe is not _SHUTIL_ORIG_UNSAFE
                or os.lstat is not _SHUTIL_ORIG_LSTAT
                or os.open is not _SHUTIL_ORIG_OPEN
                or os.fstat is not _SHUTIL_ORIG_FSTAT
                or os.path.samestat is not _SHUTIL_ORIG_SAMESTAT
                or os.scandir is not _SHUTIL_ORIG_SCANDIR
                or os.unlink is not _SHUTIL_ORIG_UNLINK
                or os.rmdir is not _SHUTIL_ORIG_RMDIR
                or os.close is not _SHUTIL_ORIG_CLOSE
                or os.fspath is not _SHUTIL_ORIG_FSPATH):
            return _shutil_orig_rmtree(
                path, ignore_errors=ignore_errors, onerror=onerror, onexc=onexc, dir_fd=dir_fd
            )
        is_str = type(path) is str
        if not is_str and not isinstance(path, os.PathLike):
            return _shutil_orig_rmtree(
                path, ignore_errors=ignore_errors, onerror=onerror, onexc=onexc, dir_fd=dir_fd
            )
        sys.audit("shutil.rmtree", path, dir_fd)
        path_arg = path if is_str else _SHUTIL_ORIG_FSPATH(path)
        if type(path_arg) is str and _shutil_posix._rmtree(path_arg, bool(ignore_errors)):
            return
        if ignore_errors:
            def _onexc(*args):
                pass
        else:
            def _onexc(*args):
                raise
        _rmtree_impl(path, dir_fd, _onexc)

    _shutil_fast_rmtree.__name__ = _shutil_orig_rmtree.__name__
    _shutil_fast_rmtree.__qualname__ = _shutil_orig_rmtree.__qualname__
    _shutil_fast_rmtree.__doc__ = _shutil_orig_rmtree.__doc__
    _shutil_fast_rmtree.avoids_symlink_attacks = _shutil_orig_rmtree.avoids_symlink_attacks
    rmtree = _shutil_fast_rmtree
