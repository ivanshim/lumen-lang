# This name is a stub; no temporary file is opened by the helper.
TESTFN = '@test'

def temp_dir(path=None, quiet=False):
    raise 'NotImplementedError: temporary directories are not supported'

def temp_cwd(name='tempcwd', quiet=False):
    raise 'NotImplementedError: changing directory is not supported'

def unlink(filename):
    raise 'NotImplementedError: file removal is not supported'

class EnvironmentVarGuard:
    def __init__(self):
        raise 'NotImplementedError: environment changes are not supported'
