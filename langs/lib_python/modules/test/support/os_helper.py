import os

# The name CPython's tests give the one scratch file they write.
TESTFN = '@test'

def temp_dir(path=None, quiet=False):
    raise 'NotImplementedError: temporary directories are not supported'

def temp_cwd(name='tempcwd', quiet=False):
    raise 'NotImplementedError: changing directory is not supported'

def unlink(filename):
    try:
        os.unlink(filename)
    except FileNotFoundError:
        pass

class EnvironmentVarGuard:
    def __init__(self):
        raise 'NotImplementedError: environment changes are not supported'
