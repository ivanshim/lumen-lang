import os

def rmtree(path, ignore_errors=False, onerror=None, *, onexc=None, dir_fd=None):
    if __remove_tree(path):
        return
    if ignore_errors:
        return
    if onexc is not None or onerror is not None:
        handler = onexc if onexc is not None else onerror
        try:
            os.listdir(path)
        except Exception as error:
            handler(rmtree, path, error)
        return
    if not os.path.exists(path):
        raise FileNotFoundError(2, 'No such file or directory', path)
    raise OSError('rmtree failed for ' + path)
