# Single-thread lock primitives, independent of the high-level threading module.
TIMEOUT_MAX = 9223372036.0

def get_ident():
    return 1

class Lock:
    def __init__(self):
        self._held = False

    def acquire(self, blocking=True, timeout=-1):
        if self._held:
            # Nothing else runs that could release it.
            if not blocking or timeout == 0:
                return False
            raise 'RuntimeError: this lock is held and no other thread can release it'
        self._held = True
        return True

    def release(self):
        if not self._held:
            raise 'RuntimeError: release unlocked lock'
        self._held = False

    def locked(self):
        return self._held

    # Entering and leaving a block is the commonest use of a lock, so the
    # two methods do what acquire and release do without calling them.
    def __enter__(self):
        if self._held:
            raise 'RuntimeError: this lock is held and no other thread can release it'
        self._held = True
        return self

    def __exit__(self, kind, value, tb):
        if not self._held:
            raise 'RuntimeError: release unlocked lock'
        self._held = False
        return False

allocate_lock = Lock

class RLock:
    def __init__(self):
        self._count = 0

    def acquire(self, blocking=True, timeout=-1):
        self._count += 1
        return True

    def release(self):
        if self._count == 0:
            raise 'RuntimeError: cannot release un-acquired lock'
        self._count -= 1

    def locked(self):
        return self._count > 0

    def _is_owned(self):
        # One thread, so the owner of a held reentrant lock is this one.
        return self._count > 0

    def __enter__(self):
        self.acquire()
        return self

    def __exit__(self, kind, value, tb):
        self.release()
        return False

error = RuntimeError
