# Host primitives used by class bodies need non-private module bindings.
_host_clock = __clock

# Wall time comes from the system clock.
def time():
    return __clock(False)

# Stub: no waiting is performed by this small library.
def sleep(seconds):
    if seconds < 0:
        raise 'ValueError: sleep length must be non-negative'
    if type(seconds) != type(0) and type(seconds) != type(0.0):
        raise 'TypeError: a number is required'

# A steady clock counts elapsed seconds from a fixed origin.
def perf_counter():
    return __clock(True)

def monotonic():
    return __clock(True)

class _ClockInfo:
    def __init__(self, steady):
        self.implementation = 'clock_gettime(CLOCK_MONOTONIC)' if steady else 'clock_gettime(CLOCK_REALTIME)'
        self.monotonic = steady
        self.adjustable = not steady
        self.resolution = _host_clock(steady, True)

def get_clock_info(name):
    if name in ('monotonic', 'perf_counter'):
        return _ClockInfo(True)
    if name == 'time':
        return _ClockInfo(False)
    raise ValueError('unknown clock')

# Integer nanoseconds from the same host clocks, without a float round trip.
def time_ns():
    return __clock(False, False)

def monotonic_ns():
    return __clock(True, False)

def perf_counter_ns():
    return __clock(True, False)

struct_time = _host_clock('struct_time')

def localtime(seconds=None):
    return _host_clock('localtime', seconds)

def gmtime(seconds=None):
    return _host_clock('gmtime', seconds)
# The host stands in the UTC zone, so the zone's figures are the
# meridian's own: no offset, no daylight time, one name.
timezone = 0
altzone = 0
daylight = 0
tzname = ('UTC', 'UTC')

_DAY_FULL = ('Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday',
             'Saturday', 'Sunday')
_DAY_SHORT = ('Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun')
_MONTH_FULL = ('', 'January', 'February', 'March', 'April', 'May', 'June',
               'July', 'August', 'September', 'October', 'November',
               'December')
_MONTH_SHORT = ('', 'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug',
                'Sep', 'Oct', 'Nov', 'Dec')

_DAYS_IN_MONTH = (-1, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
_DAYS_BEFORE_MONTH = (-1, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334)

# 1970-01-01, with 0001-01-01 as day 1 of the proleptic Gregorian
# calendar.
_EPOCH_ORD = 719163

def _leap(year):
    return divmod(year, 4)[1] == 0 and (divmod(year, 100)[1] != 0 or divmod(year, 400)[1] == 0)

def _days_before_year(year):
    earlier = year - 1
    return earlier * 365 + divmod(earlier, 4)[0] - divmod(earlier, 100)[0] + divmod(earlier, 400)[0]

def _days_before_month(year, month):
    extra = 1 if month > 2 and _leap(year) else 0
    return _DAYS_BEFORE_MONTH[month] + extra

def _ymd2ord(year, month, day):
    return _days_before_year(year) + _days_before_month(year, month) + day

def _ord2ymd(ordinal):
    # The inverse, estimated and then corrected a month either way, the
    # way the reference's date module does it. Every figure here is
    # positive, so the division's rounding never comes into it.
    remaining = ordinal - 1
    four_hundred, remaining = divmod(remaining, 146097)
    year = four_hundred * 400 + 1
    hundred, remaining = divmod(remaining, 36524)
    four, remaining = divmod(remaining, 1461)
    one, remaining = divmod(remaining, 365)
    year += hundred * 100 + four * 4 + one
    if one == 4 or hundred == 4:
        return (year - 1, 12, 31)
    leap = one == 3 and (four != 24 or hundred == 3)
    month = divmod(remaining + 50, 32)[0]
    before = _DAYS_BEFORE_MONTH[month] + (1 if month > 2 and leap else 0)
    if before > remaining:
        month -= 1
        before -= _DAYS_IN_MONTH[month] + (1 if month == 2 and leap else 0)
    return (year, month, remaining - before + 1)

def _break_down(seconds):
    # divmod floors here, so a moment before the epoch lands on the
    # right day rather than the one across midnight.
    days, left = divmod(seconds, 86400)
    hours, left = divmod(left, 3600)
    minutes, secs = divmod(left, 60)
    year, month, day = _ord2ymd(_EPOCH_ORD + int(days))
    # The host's C library keeps the year in an int and says so when a
    # moment lands outside one, the very answer the reference gives for
    # a timestamp no platform can name.
    if year > 2147483647 or year < -2147483647:
        raise OverflowError('timestamp out of range for platform time_t')
    weekday = divmod(int(days) + 3, 7)[1]
    day_of_year = _days_before_month(year, month) + day
    return (year, month, day, int(hours), int(minutes), int(secs),
            weekday, day_of_year, 0)

def mktime(fields):
    parts = list(fields)
    if len(parts) != 9:
        raise TypeError('mktime() requires a 9-sequence, not ' + type(fields).__name__)
    days = _ymd2ord(parts[0], parts[1], parts[2]) - _EPOCH_ORD
    return float(days * 86400 + parts[3] * 3600 + parts[4] * 60 + parts[5])

def _pair(number):
    word = str(number)
    return '0' + word if number < 10 else word

def _triple(number):
    word = str(number)
    if number < 10:
        return '00' + word
    return '0' + word if number < 100 else word

def _spread(number):
    word = str(number)
    return ' ' + word if number < 10 else word

def _iso_year_weeks(year):
    # A year has fifty-three ISO weeks when it opens on a Thursday, or
    # on a Wednesday it can leap past.
    first = divmod(_ymd2ord(year, 1, 1) - _EPOCH_ORD + 3, 7)[1]
    if first == 3 or (first == 2 and _leap(year)):
        return 53
    return 52

def _iso(year, weekday, day_of_year):
    # The ISO year, week and weekday of a date; day_of_year counts from
    # one and weekday from Monday as zero.
    week = divmod(day_of_year - weekday + 9, 7)[0]
    if week < 1:
        return (year - 1, _iso_year_weeks(year - 1), weekday + 1)
    if week > _iso_year_weeks(year):
        return (year + 1, 1, weekday + 1)
    return (year, week, weekday + 1)

def asctime(fields=None):
    if fields is None:
        fields = localtime()
    parts = list(fields)
    return (_DAY_SHORT[parts[6]] + ' ' + _MONTH_SHORT[parts[1]] + ' ' +
            _spread(parts[2]) + ' ' + _pair(parts[3]) + ':' + _pair(parts[4]) +
            ':' + _pair(parts[5]) + ' ' + str(parts[0]))

def ctime(seconds=None):
    return asctime(localtime(seconds))

def strftime(format, fields=None):
    # Directive by directive in the C locale, padded the way the host's
    # C library pads: a bare year is not spread to four figures, and a
    # conversion it would not know stands as written.
    if fields is None:
        fields = localtime()
    parts = list(fields)
    if len(parts) != 9:
        raise TypeError('strftime() argument must be a 9-sequence, not ' + str(len(parts)) + '-sequence')
    year, month, day, hour, minute, second, weekday, day_of_year = parts[:8]
    iso = None
    out = ''
    at = 0
    while at < len(format):
        ch = format[at]
        at += 1
        if ch != '%':
            out += ch
            continue
        if at >= len(format):
            out += '%'
            break
        ch = format[at]
        at += 1
        if ch == 'E' or ch == 'O':
            if ch == 'O' and at < len(format) and format[at] in ('B', 'b'):
                word = format[at]
                at += 1
                out += _MONTH_FULL[month] if word == 'B' else _MONTH_SHORT[month]
            else:
                out += '%' + ch
            continue
        if ch == 'a':
            out += _DAY_SHORT[weekday]
        elif ch == 'A':
            out += _DAY_FULL[weekday]
        elif ch == 'b' or ch == 'h':
            out += _MONTH_SHORT[month]
        elif ch == 'B':
            out += _MONTH_FULL[month]
        elif ch == 'c':
            out += asctime(parts)
        elif ch == 'C':
            out += str(divmod(year, 100)[0])
        elif ch == 'd':
            out += _pair(day)
        elif ch == 'e':
            out += _spread(day)
        elif ch == 'D' or ch == 'x':
            out += _pair(month) + '/' + _pair(day) + '/' + _pair(divmod(year, 100)[1])
        elif ch == 'F':
            out += str(year) + '-' + _pair(month) + '-' + _pair(day)
        elif ch == 'G':
            if iso is None:
                iso = _iso(year, weekday, day_of_year)
            out += str(iso[0])
        elif ch == 'g':
            if iso is None:
                iso = _iso(year, weekday, day_of_year)
            out += _pair(divmod(iso[0], 100)[1])
        elif ch == 'H':
            out += _pair(hour)
        elif ch == 'I':
            twelve = divmod(hour, 12)[1]
            out += _pair(12 if twelve == 0 else twelve)
        elif ch == 'j':
            out += _triple(day_of_year)
        elif ch == 'k':
            out += _spread(hour)
        elif ch == 'l':
            twelve = divmod(hour, 12)[1]
            out += _spread(12 if twelve == 0 else twelve)
        elif ch == 'm':
            out += _pair(month)
        elif ch == 'M':
            out += _pair(minute)
        elif ch == 'n':
            out += '\n'
        elif ch == 'p':
            out += 'AM' if hour < 12 else 'PM'
        elif ch == 'P':
            out += 'am' if hour < 12 else 'pm'
        elif ch == 'r':
            twelve = divmod(hour, 12)[1]
            out += (_pair(12 if twelve == 0 else twelve) + ':' + _pair(minute) +
                    ':' + _pair(second) + ' ' + ('AM' if hour < 12 else 'PM'))
        elif ch == 'R':
            out += _pair(hour) + ':' + _pair(minute)
        elif ch == 's':
            days = _ymd2ord(year, month, day) - _EPOCH_ORD
            out += str(days * 86400 + hour * 3600 + minute * 60 + second)
        elif ch == 'S':
            out += _pair(second)
        elif ch == 't':
            out += '\t'
        elif ch == 'T' or ch == 'X':
            out += _pair(hour) + ':' + _pair(minute) + ':' + _pair(second)
        elif ch == 'u':
            out += str(weekday + 1)
        elif ch == 'U':
            sunday = divmod(weekday + 1, 7)[1]
            out += _pair(divmod(day_of_year - 1 + 7 - sunday, 7)[0])
        elif ch == 'V':
            if iso is None:
                iso = _iso(year, weekday, day_of_year)
            out += _pair(iso[1])
        elif ch == 'w':
            out += str(divmod(weekday + 1, 7)[1])
        elif ch == 'W':
            out += _pair(divmod(day_of_year - 1 + 7 - weekday, 7)[0])
        elif ch == 'y':
            out += _pair(divmod(year, 100)[1])
        elif ch == 'Y':
            out += str(year)
        elif ch == 'z':
            out += '+0000'
        elif ch == 'Z':
            # A broken-down time carrying a zone name keeps it; one
            # made by hand is named for the zone when it says it is not
            # in daylight time, and nameless when it cannot say.
            zone = getattr(fields, 'tm_zone', None)
            if zone is not None:
                out += zone
            elif parts[8] >= 0:
                out += 'UTC'
        elif ch == '%':
            out += '%'
        else:
            out += '%' + ch
    return out

# The system clock exposes the native C calendar formatter.
ctime = _host_clock("ctime")
