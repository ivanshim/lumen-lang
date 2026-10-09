# Expose coroutine inspection independently of unavailable event-loop services.
# Event loops, networking, scheduling, and their APIs remain unavailable.
from . import coroutines
from .coroutines import iscoroutine, iscoroutinefunction

__all__ = ('iscoroutine', 'iscoroutinefunction')
