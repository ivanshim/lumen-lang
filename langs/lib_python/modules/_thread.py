# One thread runs here, so the pieces a single thread can honestly have
# are the ones threading already defines: an identifier and a lock that
# is never contended.
from threading import Lock as allocate_lock
from threading import get_ident

error = RuntimeError
