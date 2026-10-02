# Source: CPython Lib/datetime.py at v3.14.8 / 8e6e75d9102e; PSF License.
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
