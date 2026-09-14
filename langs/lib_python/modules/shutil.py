# Removing a tree means listing the directories in it, and os.listdir is not
# available here, so rmtree can clear nothing. It says so when asked to try,
# and honours ignore_errors, which is the one promise it can keep.
import os

def rmtree(path, ignore_errors=False, onerror=None, *, onexc=None, dir_fd=None):
    if ignore_errors:
        return
    if onexc is not None or onerror is not None:
        handler = onexc if onexc is not None else onerror
        try:
            os.listdir(path)
        except Exception as error:
            handler(rmtree, path, error)
        return
    raise 'NotImplementedError: shutil.rmtree cannot list a directory here'
