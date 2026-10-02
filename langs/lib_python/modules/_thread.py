# Lock primitives for the interpreter's single execution thread.
import os

def get_ident():
    return os.getpid()

def _check_timeout(blocking, timeout):
    if not isinstance(timeout, (int, float)):
        raise TypeError('timeout must be a number')
    if not blocking and timeout != -1:
        raise ValueError("can't specify a timeout for a non-blocking call")
    if timeout < 0 and timeout != -1:
        raise ValueError('timeout value must be positive')
    if timeout != -1:
        raise NotImplementedError('lock timeouts are not supported')

class LockType:
    def __init__(self):
        self._held = False
    def acquire(self, blocking=True, timeout=-1):
        _check_timeout(blocking, timeout)
        if self._held:
            if not blocking:
                return False
            raise NotImplementedError('blocking an execution thread is not supported')
        self._held = True
        return True
    def release(self):
        if not self._held:
            raise RuntimeError('release unlocked lock')
        self._held = False
    def locked(self):
        return self._held
    def __enter__(self):
        return self.acquire()
    def __exit__(self, *args):
        self.release()

class RLock(LockType):
    def __init__(self):
        self._count = 0
    def acquire(self, blocking=True, timeout=-1):
        _check_timeout(blocking, timeout)
        self._count += 1
        return True
    def release(self):
        if not self._count:
            raise RuntimeError('cannot release un-acquired lock')
        self._count -= 1
    def locked(self):
        return bool(self._count)

def allocate_lock():
    return LockType()
