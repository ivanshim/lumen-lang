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

# A guard over os.environ: it remembers each name's value the first
# time a test changes or removes it, and puts every one of them back
# when the guard closes. The mapping it edits lives inside this run
# alone; nothing written here reaches the host process.
class EnvironmentVarGuard:
    def __init__(self):
        self._environ = os.environ
        self._changed = {}

    def __getitem__(self, envvar):
        return self._environ[envvar]

    def __setitem__(self, envvar, value):
        if envvar not in self._changed:
            self._changed[envvar] = self._environ.get(envvar)
        self._environ[envvar] = value

    def __delitem__(self, envvar):
        if envvar not in self._changed:
            self._changed[envvar] = self._environ.get(envvar)
        if envvar in self._environ:
            del self._environ[envvar]

    def keys(self):
        return self._environ.keys()

    def __contains__(self, envvar):
        return envvar in self._environ

    def __iter__(self):
        return iter(self._environ)

    def __len__(self):
        return len(self._environ)

    def set(self, envvar, value):
        self[envvar] = value

    def unset(self, envvar):
        del self[envvar]

    def copy(self):
        return dict(self._environ)

    def __enter__(self):
        return self

    def __exit__(self, *ignore_exc):
        for key, value in self._changed.items():
            if value is None:
                if key in self._environ:
                    del self._environ[key]
            else:
                self._environ[key] = value
        return False
