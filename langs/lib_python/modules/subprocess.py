# A second interpreter, started beside this one and read through pipes.
# The kernel's own word (__subprocess) does the work in steps — begin a
# run, write into its input, close that input, read a stream, wait for
# its end, ask whether it has ended, or stop it — and this module turns
# those steps into the Popen and run that CPython's tests reach for. A
# begun run is named by a whole number the begin step hands back.

_subprocess = __subprocess
_wait = __wait

PIPE = -1
STDOUT = -2
DEVNULL = -3


class SubprocessError(Exception):
    pass


class TimeoutExpired(SubprocessError):
    def __init__(self, cmd, timeout, output=None, stderr=None):
        self.cmd = cmd
        self.timeout = timeout
        self.output = output
        self.stderr = stderr

    def __str__(self):
        return "Command '%s' timed out after %s seconds" % (self.cmd, self.timeout)


class CalledProcessError(SubprocessError):
    def __init__(self, returncode, cmd, output=None, stderr=None):
        self.returncode = returncode
        self.cmd = cmd
        self.output = output
        self.stderr = stderr

    def __str__(self):
        return "Command '%s' returned non-zero exit status %d." % (self.cmd, self.returncode)


def _stream_mode(stream, is_stderr=False):
    if stream == PIPE:
        return 1
    if stream == DEVNULL:
        return 0
    if stream == STDOUT and is_stderr:
        return 3
    return 2


class _Readable:
    def __init__(self, handle, which):
        self._handle = handle
        self._which = which
        self.closed = False

    def read(self, size=-1):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        result = _subprocess(3, self._handle, self._which)
        if result is None or result is False:
            return b''
        return result

    def close(self):
        self.closed = True


class _Writable:
    def __init__(self, handle):
        self._handle = handle
        self.closed = False

    def write(self, data):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        return _subprocess(1, self._handle, data)

    def close(self):
        if not self.closed:
            _subprocess(2, self._handle)
            self.closed = True


class Popen:
    def __init__(self, args, bufsize=-1, executable=None, stdin=None, stdout=None,
                 stderr=None, preexec_fn=None, close_fds=True, shell=False, cwd=None,
                 env=None, universal_newlines=None, startupinfo=None, creationflags=0,
                 restore_signals=True, start_new_session=False, pass_fds=(), *,
                 user=None, group=None, extra_groups=None, encoding=None, errors=None,
                 text=None, umask=-1, pipesize=-1, process_group=None):
        if shell:
            raise NotImplementedError('subprocess: shell=True is not supported')
        if isinstance(args, str):
            argv = [args]
        else:
            argv = list(args)
        if env is None:
            env = {}
        in_mode = _stream_mode(stdin)
        out_mode = _stream_mode(stdout)
        err_mode = _stream_mode(stderr, True)
        handle = _subprocess(0, argv, env, in_mode, out_mode, err_mode)
        if handle is False:
            raise OSError(2, 'No such file or directory', argv[0])
        self._handle = handle
        self._stderr_to_stdout = stderr == STDOUT
        self.args = argv
        self.returncode = None
        self.stdin = _Writable(handle) if in_mode == 1 else None
        self.stdout = _Readable(handle, 0) if out_mode == 1 else None
        self.stderr = None if self._stderr_to_stdout else (_Readable(handle, 1) if err_mode in (1, 3) else None)

    def poll(self):
        if self.returncode is not None:
            return self.returncode
        code = _subprocess(5, self._handle)
        if code is not None and code is not False:
            self.returncode = code
            return code
        return None

    def wait(self, timeout=None):
        if self.returncode is not None:
            return self.returncode
        if timeout is None:
            code = _subprocess(4, self._handle)
            if code is False:
                return None
            self.returncode = code
            return code
        import time
        deadline = time.monotonic() + timeout
        while True:
            code = _subprocess(5, self._handle)
            if code is not None and code is not False:
                self.returncode = code
                return code
            if time.monotonic() >= deadline:
                self.kill()
                raise TimeoutExpired(self.args, timeout)
            _wait(10000)

    def _read_stream(self, which):
        result = _subprocess(3, self._handle, which)
        if result is None or result is False:
            return b''
        return result

    def communicate(self, input=None, timeout=None):
        if input is not None and self.stdin is None:
            raise ValueError('stdin is not a pipe')
        if input is not None:
            self.stdin.write(input)
        if self.stdin is not None:
            self.stdin.close()
            self.stdin = None
        if timeout is not None:
            import time
            deadline = time.monotonic() + timeout
            while True:
                code = _subprocess(5, self._handle)
                if code is not None and code is not False:
                    self.returncode = code
                    break
                if time.monotonic() >= deadline:
                    self.kill()
                    raise TimeoutExpired(self.args, timeout)
                _wait(10000)
        out = self.stdout.read() if self.stdout is not None else None
        if self._stderr_to_stdout:
            if self.stdout is not None:
                out = out + self._read_stream(1)
            err = None
        else:
            err = self.stderr.read() if self.stderr is not None else None
        if self.returncode is None:
            self.returncode = self.wait()
        return (out, err)

    def kill(self):
        _subprocess(6, self._handle)
        self.returncode = -9

    def terminate(self):
        self.kill()

    def send_signal(self, sig):
        self.kill()

    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        if self.stdin is not None:
            self.stdin.close()
        if self.stdout is not None:
            self.stdout.close()
        if self.stderr is not None:
            self.stderr.close()
        self.wait()
        return False


class CompletedProcess:
    def __init__(self, args, returncode, stdout=None, stderr=None):
        self.args = args
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr

    def __repr__(self):
        args = ['%r' % a for a in self.args]
        return '%s(%s, %r, stdout=%r, stderr=%r)' % (self.__class__.__name__, args, self.returncode, self.stdout, self.stderr)

    def check_returncode(self):
        if self.returncode:
            raise CalledProcessError(self.returncode, self.args, self.stdout, self.stderr)


def _cleanup():
    pass


def run(*popenargs, input=None, capture_output=False, timeout=None, check=False, **kwargs):
    if input is not None:
        if kwargs.get('stdin') is not None:
            raise ValueError('stdin and input arguments may not both be used.')
        kwargs['stdin'] = PIPE
    if capture_output:
        if kwargs.get('stdout') is not None or kwargs.get('stderr') is not None:
            raise ValueError('stdout and stderr arguments may not be used with capture_output.')
        kwargs['stdout'] = PIPE
        kwargs['stderr'] = PIPE
    with Popen(*popenargs, **kwargs) as process:
        try:
            stdout, stderr = process.communicate(input, timeout=timeout)
        except TimeoutExpired as exc:
            process.kill()
            raise
        except Exception:
            process.kill()
            raise
        retcode = process.poll()
        if check and retcode:
            raise CalledProcessError(retcode, process.args, output=stdout, stderr=stderr)
    return CompletedProcess(process.args, retcode, stdout, stderr)


def check_output(*popenargs, timeout=None, **kwargs):
    if 'stdout' in kwargs:
        raise ValueError('stdout argument not allowed, it will be overridden.')
    if 'input' in kwargs and kwargs['input'] is None:
        del kwargs['input']
    return run(*popenargs, stdout=PIPE, timeout=timeout, check=True, **kwargs).stdout


def call(*popenargs, timeout=None, **kwargs):
    with Popen(*popenargs, **kwargs) as p:
        return p.wait(timeout=timeout)


def check_call(*popenargs, **kwargs):
    retcode = call(*popenargs, **kwargs)
    if retcode:
        cmd = kwargs.get('args')
        if cmd is None:
            cmd = popenargs[0]
        raise CalledProcessError(retcode, cmd)
    return 0
