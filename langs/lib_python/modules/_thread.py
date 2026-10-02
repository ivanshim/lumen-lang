# Lock primitives for the interpreter's single execution thread.
import os

def get_ident():
    return os.getpid()

class LockType:
    def __init__(self):
        self._held = False
    def acquire(self, blocking=True, timeout=-1):
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
        self.acquire()
        return self
    def __exit__(self, *args):
        self.release()

class RLock(LockType):
    def __init__(self):
        self._count = 0
    def acquire(self, blocking=True, timeout=-1):
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
