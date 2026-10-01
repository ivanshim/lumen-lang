# From CPython 3.14, Lib/datetime.py.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# The PSF license is kept in tests/python/LICENSE.
"""Specific date/time and related types.

See https://data.iana.org/time-zones/tz-link.html for
time zone and DST data sources.
"""

try:
    from _datetime import *
except ImportError:
    from _pydatetime import *

__all__ = ("date", "datetime", "time", "timedelta", "timezone", "tzinfo",
           "MINYEAR", "MAXYEAR", "UTC")
