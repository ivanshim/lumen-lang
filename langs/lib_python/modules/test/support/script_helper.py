# Helpers that run a second interpreter. The host runs a file, so the
# CPython-style arguments a test hands over (options, a `-c` with code,
# a file) are turned into a file to run plus the program's own words;
# options that mean nothing to this host are dropped, and `-c code` is
# written to a scratch file first.

import os
import subprocess
import sys
import tempfile


def _to_file_args(args):
    args = list(args)
    file = None
    rest = []
    i = 0
    n = len(args)
    while i < n:
        a = args[i]
        if a == '-c':
            code = args[i + 1]
            directory = tempfile.mkdtemp(prefix='script_helper_')
            file = os.path.join(directory, 'code.py')
            with open(file, 'w', encoding='utf-8') as out:
                out.write(code)
            i += 2
        elif a == '-m' and i + 1 < n:
            file = '-m'
            rest = args[i + 1:]
            break
        elif a in ('-X', '-W', '-Q') and i + 1 < n:
            i += 2
        elif len(a) > 2 and a[:2] in ('-X', '-W', '-Q'):
            i += 1
        elif a in ('-E', '-I', '-s', '-S', '-u', '-B', '-d', '-v', '-q',
                   '-O', '-OO', '-P', '-R', '-t', '-tt', '-b', '-bb', '-i', '-x'):
            i += 1
        else:
            file = a
            rest = args[i + 1:]
            break
    return file, rest


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
    return _PythonRunResult(p.returncode, out, err)
