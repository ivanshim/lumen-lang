# Helpers that run a child interpreter with its original Python arguments.

import os
import subprocess
import sys
import tempfile

from test import support


# Preserve source strings, filenames, flags and the child's argument order.
def _to_file_args(args):
    args = list(args)
    return (args[0], args[1:]) if args else (None, [])


def _option_env(args):
    # The -X options that mean something to the library are handed to the
    # second interpreter as the environment variables that do the same.
    env = {}
    args = list(args)
    i = 0
    while i < len(args):
        a = args[i]
        if a in ('-c', '-m') or not a.startswith('-'):
            break
        if a == '-X' and i + 1 < len(args):
            option = args[i + 1]
            i += 2
        elif a.startswith('-X') and len(a) > 2:
            option = a[2:]
            i += 1
        else:
            i += 1
            continue
        if option == 'dev':
            env['PYTHONDEVMODE'] = '1'
        elif option in ('utf8', 'utf8=1'):
            env['PYTHONUTF8'] = '1'
        elif option == 'utf8=0':
            env['PYTHONUTF8'] = '0'
        elif option == 'warn_default_encoding':
            env['PYTHONWARNDEFAULTENCODING'] = '1'
    return env


def make_script(script_dir, script_basename, source, omit_suffix=False):
    if omit_suffix:
        script_filename = script_basename
    else:
        script_filename = script_basename + os.extsep + 'py'
    script_name = os.path.join(script_dir, script_filename)
    if isinstance(source, str):
        # Text is written in UTF-8, the encoding a file is read in by
        # default; anything already in bytes is written as it came.
        with open(script_name, 'w', encoding='utf-8') as script_file:
            script_file.write(source)
    else:
        with open(script_name, 'wb') as script_file:
            script_file.write(source)
    return script_name


def make_zip_script(zip_dir, zip_basename, script_name, name_in_zip=None):
    import zipfile
    zip_filename = zip_basename + os.extsep + 'zip'
    zip_name = os.path.join(zip_dir, zip_filename)
    with zipfile.ZipFile(zip_name, 'w') as zip_file:
        if name_in_zip is None:
            parts = script_name.split(os.sep)
            if len(parts) >= 2 and parts[-2] == '__pycache__':
                from importlib.util import source_from_cache
                from test.support.import_helper import make_legacy_pyc
                legacy_pyc = make_legacy_pyc(source_from_cache(script_name))
                name_in_zip = os.path.basename(legacy_pyc)
                script_name = legacy_pyc
            else:
                name_in_zip = os.path.basename(script_name)
        zip_file.write(script_name, name_in_zip)
    return zip_name, os.path.join(zip_name, name_in_zip)


class _PythonRunResult:
    """Helper for reporting Python subprocess run results."""

    def __init__(self, rc, out, err):
        self.rc = rc
        self.out = out
        self.err = err

    def __iter__(self):
        yield self.rc
        yield self.out
        yield self.err

    def __len__(self):
        return 3

    def __getitem__(self, index):
        return (self.rc, self.out, self.err)[index]

    def __repr__(self):
        return '_PythonRunResult(%r, %r, %r)' % (self.rc, self.out, self.err)

    def __eq__(self, other):
        if isinstance(other, (_PythonRunResult, tuple)):
            return (self.rc, self.out, self.err) == tuple(other)
        return NotImplemented

    def __ne__(self, other):
        equal = self.__eq__(other)
        if equal is NotImplemented:
            return NotImplemented
        return not equal

    def __hash__(self):
        return hash((self.rc, self.out, self.err))


def _assert_python(expected_success, *args, **env_vars):
    file, rest = _to_file_args(args)
    cmd_line = [sys.executable, file] + rest
    env = os.environ.copy()
    env.update(_option_env(args))
    env.update(env_vars)
    p = subprocess.Popen(cmd_line, stdin=subprocess.PIPE,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         env=env)
    try:
        out, err = p.communicate()
    finally:
        subprocess._cleanup()
        p.stdout.close()
        p.stderr.close()
    rc = p.returncode
    if (rc and expected_success) or (not rc and not expected_success):
        raise AssertionError(
            'Process return code is %s\n'
            'command line: %s\n'
            'stdout:\n'
            '---\n'
            '%s\n'
            '---\n'
            'stderr:\n'
            '---\n'
            '%s\n'
            '---' % (rc, cmd_line, out, err))
    return _PythonRunResult(rc, out, err)


def assert_python_ok(*args, **env_vars):
    return _assert_python(True, *args, **env_vars)


def assert_python_failure(*args, **env_vars):
    return _assert_python(False, *args, **env_vars)


def spawn_python(*args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, **kw):
    file, rest = _to_file_args(args)
    cmd_line = [sys.executable, file] + rest
    env = os.environ.copy()
    env.update(_option_env(args))
    env['PYTHONPATH'] = os.pathsep.join(filter(None, sys.path))
    kw['env'] = env
    kw.setdefault('stdin', subprocess.PIPE)
    kw['stdout'] = stdout
    kw['stderr'] = stderr
    return subprocess.Popen(cmd_line, **kw)


def run_python_until_end(*args, **env_vars):
    file, rest = _to_file_args(args)
    cmd_line = [sys.executable, file] + rest
    env = os.environ.copy()
    env.update(_option_env(args))
    env.update(env_vars)
    p = subprocess.Popen(cmd_line, stdin=subprocess.PIPE,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         env=env)
    try:
        out, err = p.communicate()
    finally:
        subprocess._cleanup()
        p.stdout.close()
        p.stderr.close()
    return _PythonRunResult(p.returncode, out, err), cmd_line


@support.requires_subprocess()
def run_test_script(script):
    # use -u to try to get the full output if the test hangs or crash
    if support.verbose:
        def title(text):
            return f"===== {text} ======"

        name = f"script {os.path.basename(script)}"
        print()
        print(title(name), flush=True)
        # In verbose mode, the child process inherit stdout and stdout,
        # to see output in realtime and reduce the risk of losing output.
        args = [sys.executable, "-E", "-X", "faulthandler", "-u", script, "-v"]
        proc = subprocess.run(args)
        print(title(f"{name} completed: exit code {proc.returncode}"),
              flush=True)
        if proc.returncode:
            raise AssertionError(f"{name} failed")
    else:
        assert_python_ok("-u", script, "-v")

# Drain and reap a child after its input has been closed.
def kill_python(p):
    p.stdin.close()
    data = p.stdout.read()
    p.stdout.close()
    p.wait()
    subprocess._cleanup()
    return data
