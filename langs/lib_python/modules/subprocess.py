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


# A pipe opened for text has each result decoded and its line endings
# normalised: CRLF and a lone CR both become the one LF, the universal
# newline rule the reference applies while reading. Binary pipes keep
# their bytes untouched.
def _text_of(data, encoding, errors):
    text = data.decode(encoding, errors or 'strict')
    if '\r' in text:
        text = text.replace('\r\n', '\n').replace('\r', '\n')
    return text


class _Readable:
    def __init__(self, handle, which, text=False, encoding='utf-8', errors=None):
        self._handle = handle
        self._which = which
        self._text = text
        self._encoding = encoding
        self._errors = errors
        self.encoding = encoding if text else None
        self.errors = errors if text else None
        self.closed = False

    # Decode captured bytes and apply universal newline translation.
    def read(self, size=-1):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        result = _subprocess(3, self._handle, self._which)
        if result is None or result is False:
            result = b''
        if self._text:
            return _text_of(result, self._encoding, self._errors)
        return result

    # Close the host pipe as well as the Python wrapper.
    def close(self):
        if not self.closed:
            _subprocess(7, self._handle, self._which)
            self.closed = True


class _Writable:
    def __init__(self, handle, text=False, encoding='utf-8', errors=None):
        self._handle = handle
        self._text = text
        self._encoding = encoding
        self._errors = errors
        self.encoding = encoding if text else None
        self.errors = errors if text else None
        self.closed = False

    # Encode text writes while reporting their character count.
    def write(self, data):
        if self.closed:
            raise ValueError('I/O operation on closed file')
        written = None
        if self._text and not isinstance(data, str):
            raise TypeError('write() argument must be str, not %s' % type(data).__name__)
        if self._text:
            # A text stream counts the characters handed to it, not the
            # bytes they were written as; the caller advances by that.
            written = len(data)
            data = data.encode(self._encoding, self._errors or 'strict')
        result = _subprocess(1, self._handle, data)
        if isinstance(result, tuple):
            error, message = result
            kind = BrokenPipeError if error == 32 else OSError
            raise kind(error, message)
        return result if written is None else written

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
        if text is not None and universal_newlines is not None and bool(text) != bool(universal_newlines):
            raise SubprocessError('Cannot disambiguate when both text and universal_newlines are supplied but different. Pass one or the other.')
        self.text_mode = bool(encoding or errors or text or universal_newlines)
        self.encoding = encoding
        self.errors = 'strict' if errors is None else errors
        if self.text_mode and self.encoding is None:
            import locale
            self.encoding = locale.getencoding()
        if shell:
            raise NotImplementedError('subprocess: shell=True is not supported')
        import os
        if isinstance(args, (str, bytes, os.PathLike)):
            argv = [os.fsdecode(args)]
        else:
            argv = [os.fsdecode(arg) for arg in args]
        if env is None:
            env = {}
        in_mode = _stream_mode(stdin)
        out_mode = _stream_mode(stdout)
        err_mode = _stream_mode(stderr, True)
        # A stream of text is asked for by either spelling, and a named
        # encoding or error handler implies it as well. The decoding is
        # done here after the bytes are read, because the kernel moves
        # raw bytes through the pipe.
        self._text = self.text_mode
        self._encoding = self.encoding or 'utf-8'
        self._errors = self.errors
        # The directory is the seventh word of the begin step, sent only when asked for.
        where = ()
        if cwd is not None:
            cwd = os.fsdecode(cwd)
            where = (cwd,)
        handle = _subprocess(0, argv, env, in_mode, out_mode, err_mode, *where)
        if handle is False:
            raise OSError(2, 'No such file or directory', cwd if cwd is not None and not os.path.isdir(cwd) else argv[0])
        self._handle = handle
        self._stderr_to_stdout = stderr == STDOUT
        self.args = args
        self.returncode = None
        self.stdin = _Writable(handle, self._text, self._encoding, self._errors) if in_mode == 1 else None
        self.stdout = _Readable(handle, 0, self._text, self._encoding, self._errors) if out_mode == 1 else None
        self.stderr = None if self._stderr_to_stdout else (_Readable(handle, 1, self._text, self._encoding, self._errors) if err_mode in (1, 3) else None)

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
            result = b''
        if self._text:
            return _text_of(result, self._encoding, self._errors)
        return result

    # Drain open captures and tolerate streams already closed by the caller.
    def communicate(self, input=None, timeout=None):
        if input is not None and self.stdin is None:
            raise ValueError('stdin is not a pipe')
        if input is not None:
            if self.stdin.closed:
                raise ValueError("I/O operation on closed file")
            try:
                self.stdin.write(input)
            except BrokenPipeError:
                pass
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
        empty = '' if self.text_mode else b''
        out = (empty if self.stdout.closed else self.stdout.read()) if self.stdout is not None else None
        if self._stderr_to_stdout:
            if self.stdout is not None:
                out = out + self._read_stream(1)
            err = None
        else:
            err = (empty if self.stderr.closed else self.stderr.read()) if self.stderr is not None else None
        if self.returncode is None:
            self.returncode = self.wait()
        if self._text:
            if isinstance(out, bytes):
                out = _text_of(out, self._encoding, self._errors)
            if isinstance(err, bytes):
                err = _text_of(err, self._encoding, self._errors)
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
