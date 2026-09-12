# One thread of control runs here and no second one can be started. The
# pieces that are honest for a single thread are real: a lock is never
# contended, so acquiring it always succeeds, and thread-local storage has
# one owner. Anything that needs a second thread to make progress -- starting
# a Thread, waiting on an Event that nothing else will set -- refuses,
# because it would otherwise wait for ever or quietly do nothing.

TIMEOUT_MAX = 9223372036.0

def get_ident():
    return 1

def get_native_id():
    return 1

def active_count():
    return 1

def main_thread():
    return _main

def current_thread():
    return _main

currentThread = current_thread
activeCount = active_count

def enumerate():
    return [_main]

def stack_size(size=None):
    if size is not None:
        raise 'NotImplementedError: no thread stacks are allocated here'
    return 0

def settrace(func):
    return None

def setprofile(func):
    return None

ThreadError = RuntimeError

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

    def __enter__(self):
        self.acquire()
        return self

    def __exit__(self, kind, value, tb):
        self.release()
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

    def __enter__(self):
        self.acquire()
        return self

    def __exit__(self, kind, value, tb):
        self.release()
        return False

class local:
    # One thread, so plain attributes already are thread-local.
    pass

class Event:
    def __init__(self):
        self._flag = False

    def is_set(self):
        return self._flag

    isSet = is_set

    def set(self):
        self._flag = True

    def clear(self):
        self._flag = False

    def wait(self, timeout=None):
        if self._flag:
            return True
        if timeout is not None:
            return False
        raise 'RuntimeError: no other thread can set this event'

class Semaphore:
    def __init__(self, value=1):
        if value < 0:
            raise 'ValueError: semaphore initial value must be >= 0'
        self._value = value

    def acquire(self, blocking=True, timeout=None):
        if self._value > 0:
            self._value -= 1
            return True
        if not blocking:
            return False
        raise 'RuntimeError: no other thread can release this semaphore'

    def release(self, n=1):
        self._value += n

    def __enter__(self):
        self.acquire()
        return self

    def __exit__(self, kind, value, tb):
        self.release()
        return False

class BoundedSemaphore(Semaphore):
    def __init__(self, value=1):
        Semaphore.__init__(self, value)
        self._initial = value

    def release(self, n=1):
        if self._value + n > self._initial:
            raise 'ValueError: Semaphore released too many times'
        self._value += n

class Condition:
    def __init__(self, lock=None):
        self._lock = lock if lock is not None else RLock()
        self.acquire = self._lock.acquire
        self.release = self._lock.release

    def __enter__(self):
        self._lock.acquire()
        return self

    def __exit__(self, kind, value, tb):
        self._lock.release()
        return False

    def wait(self, timeout=None):
        if timeout is not None:
            return False
        raise 'RuntimeError: no other thread can notify this condition'

    def wait_for(self, predicate, timeout=None):
        return predicate()

    def notify(self, n=1):
        return None

    def notify_all(self):
        return None

    notifyAll = notify_all

class Barrier:
    def __init__(self, parties, action=None, timeout=None):
        self.parties = parties
        self.n_waiting = 0
        self.broken = False

    def wait(self, timeout=None):
        if self.parties == 1:
            return 0
        raise 'RuntimeError: a barrier needs other threads to reach it'

    def reset(self):
        return None

    def abort(self):
        self.broken = True

class Thread:
    def __init__(self, group=None, target=None, name=None, args=(), kwargs=None,
                 *, daemon=None):
        if group is not None:
            raise 'AssertionError: group argument must be None for now'
        self._target = target
        self._args = args
        self._kwargs = kwargs if kwargs is not None else {}
        self._name = name if name is not None else 'Thread-0'
        self._started = False
        self.daemon = bool(daemon)

    @property
    def name(self):
        return self._name

    @property
    def ident(self):
        return 1 if self._started else None

    @property
    def native_id(self):
        return 1 if self._started else None

    def is_alive(self):
        return False

    isAlive = is_alive

    def start(self):
        raise 'RuntimeError: no worker thread can be started here'

    def run(self):
        if self._target is not None:
            self._target(*self._args, **self._kwargs)

    def join(self, timeout=None):
        if self._started:
            return None
        raise 'RuntimeError: cannot join a thread that was never started'

    def __repr__(self):
        return '<Thread(' + self._name + ', initial)>'

class _MainThread(Thread):
    def __init__(self):
        Thread.__init__(self, name='MainThread')
        self._started = True

    def is_alive(self):
        return True

    def start(self):
        raise 'RuntimeError: threads can only be started once'

    def join(self, timeout=None):
        raise 'RuntimeError: cannot join current thread'

    def __repr__(self):
        return '<_MainThread(MainThread, started 1)>'

_main = _MainThread()

class Timer(Thread):
    def __init__(self, interval, function, args=None, kwargs=None):
        Thread.__init__(self, target=function,
                        args=args if args is not None else (),
                        kwargs=kwargs)
        self.interval = interval
        self.finished = Event()

    def cancel(self):
        self.finished.set()
