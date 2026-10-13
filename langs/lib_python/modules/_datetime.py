# Expose the Python datetime implementation through the extension import name.
# The versioned native C interface retains these exact Python classes.
from _pydatetime import date, datetime, time, timedelta, timezone, tzinfo, MINYEAR, MAXYEAR, UTC
__all__ = ('date', 'datetime', 'time', 'timedelta', 'timezone', 'tzinfo', 'MINYEAR', 'MAXYEAR', 'UTC')

datetime_CAPI = __lumen_datetime_capi__(date, datetime, time, timedelta, tzinfo, timezone, UTC)
__all__ += ("datetime_CAPI",)
