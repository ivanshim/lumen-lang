# The single-thread primitives shared with threading. Starting threads remains
# unavailable; these locks and the current thread identity are implemented there.
from threading import RLock, Lock, get_ident, TIMEOUT_MAX
allocate_lock = Lock
error = RuntimeError
