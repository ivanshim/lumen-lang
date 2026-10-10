# Expose the Python datetime implementation through the extension import name.
# No C capsule is exported: this interpreter has no CPython datetime C API.
from _pydatetime import date, datetime, time, timedelta, timezone, tzinfo, MINYEAR, MAXYEAR, UTC
__all__ = ('date', 'datetime', 'time', 'timedelta', 'timezone', 'tzinfo', 'MINYEAR', 'MAXYEAR', 'UTC')
