# Keep native bridges in the retained namespace when source loaders reload us.
try:
    _host_clock
except NameError:
    _host_clock = __clock
try:
    _host_wait
except NameError:
    _host_wait = __wait

# Wall time comes from the system clock.
def time():
    return _host_clock(False)

# Wait on the host and allow signal handlers to interrupt the wait.
def sleep(seconds):
    if not isinstance(seconds, (int, float)):
        raise TypeError("'" + type(seconds).__name__ + "' object cannot be interpreted as an integer or float")
    if seconds < 0:
        raise ValueError('sleep length must be non-negative')
    return _host_wait(int(seconds * 1000000 + 0.5))

# A steady clock counts elapsed seconds from a fixed origin.
def perf_counter():
    return _host_clock(True)

# Read elapsed seconds from the steady clock.
def monotonic():
    return _host_clock(True)

class _ClockInfo:
    def __init__(self, name, steady):
        self.name = name
        if name == 'process_time':
            self.implementation = 'clock_gettime(CLOCK_PROCESS_CPUTIME_ID)'
            self.resolution = _host_clock('process_time', True)
        elif name == 'thread_time':
            self.implementation = 'clock_gettime(CLOCK_THREAD_CPUTIME_ID)'
            self.resolution = _host_clock('thread_time', True)
        else:
            self.implementation = 'clock_gettime(CLOCK_MONOTONIC)' if steady else 'clock_gettime(CLOCK_REALTIME)'
            self.resolution = _host_clock(steady, True)
        self.monotonic = steady
        self.adjustable = not steady

def get_clock_info(name):
    if name in ('monotonic', 'perf_counter'):
        return _ClockInfo(name, True)
    if name == 'time':
        return _ClockInfo(name, False)
    if name in ('process_time', 'thread_time'):
        return _ClockInfo(name, True)
    raise ValueError('unknown clock')

# Integer nanoseconds from the same host clocks, without a float round trip.
def time_ns():
    return _host_clock(False, False)

# Return the monotonic clock without a float conversion.
def monotonic_ns():
    return _host_clock(True, False)

# Return high-resolution elapsed time in integer nanoseconds.
def perf_counter_ns():
    return _host_clock(True, False)

# Processor time spent by the whole run, and by this thread alone, in
# seconds and in whole nanoseconds. Neither counts time spent waiting.
def process_time():
    return __clock('process_time')

def process_time_ns():
    return __clock('process_time', False)

def thread_time():
    return __clock('thread_time')

def thread_time_ns():
    return __clock('thread_time', False)

# The count of fields a struct_time carries in this build, zone and
# offset included.
_STRUCT_TM_ITEMS = 11

struct_time = _host_clock('struct_time')

def localtime(seconds=None):
    return _host_clock('localtime', seconds)

def gmtime(seconds=None):
    return _host_clock('gmtime', seconds)
def tzset():
    # Publish the four figures the host tzset() just wrote.
    figures = _host_clock('zone')
    global timezone, altzone, daylight, tzname
    timezone = figures[0]
    altzone = figures[1]
    daylight = figures[2]
    tzname = (figures[3], figures[4])

timezone = 0
altzone = 0
daylight = 0
tzname = ('UTC', 'UTC')
tzset()

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

def _c_rem(value, base):
    # The C library's remainder keeps the sign of what it divides, while
    # this language's own always lands a step forward.
    rest = abs(value) % base
    return rest if value >= 0 else -rest

def _tm_checked(fields):
    # The year and the fields of a time tuple, held to the ranges the
    # reference's C library holds them to; zero stands for the lowest
    # useable figure in the same places it does there. The weekday comes
    # back counted from Monday, as the naming tables here expect.
    parts = list(fields)
    if len(parts) != 9:
        raise TypeError('a 9-sequence is required, not a ' + str(len(parts)) + '-sequence')
    year = parts[0]
    if year > 2147483647 or year < -2147483648 + 1900:
        raise OverflowError('year out of range')
    month = parts[1]
    if month == 0:
        month = 1
    elif month < 0 or month > 12:
        raise ValueError('month out of range')
    day = parts[2]
    if day == 0:
        day = 1
    elif day < 0 or day > 31:
        raise ValueError('day of month out of range')
    hour = parts[3]
    if hour < 0 or hour > 23:
        raise ValueError('hour out of range')
    minute = parts[4]
    if minute < 0 or minute > 59:
        raise ValueError('minute out of range')
    second = parts[5]
    if second < 0 or second > 61:
        raise ValueError('seconds out of range')
    counted = _c_rem(parts[6] + 1, 7)
    if counted < 0:
        raise ValueError('day of week out of range')
    weekday = (counted + 6) % 7
    day_of_year = parts[7]
    if day_of_year == 0:
        day_of_year = 1
    elif day_of_year < 0 or day_of_year > 366:
        raise ValueError('day of year out of range')
    return (year, month, day, hour, minute, second, weekday, day_of_year, parts[8])

def asctime(fields=None):
    if fields is None:
        fields = localtime()
    parts = _tm_checked(fields)
    return (_DAY_SHORT[parts[6]] + ' ' + _MONTH_SHORT[parts[1]] + ' ' +
            _spread(parts[2]) + ' ' + _pair(parts[3]) + ':' + _pair(parts[4]) +
            ':' + _pair(parts[5]) + ' ' + str(parts[0]))

def ctime(seconds=None):
    return asctime(localtime(seconds))

# The E and O modifiers name an alternative representation the C locale
# does not carry, so over the plain conversion they fall back on it; the
# pairs the host's C library refuses stand as written instead.
_E_MODIFIERS = 'cCnpPrRstTuXxYyZz%'
_O_MODIFIERS = 'bBCdegGhHIjklmMnpPrRsStTuUVwWyzZ%'

def _modifier_fits(modifier, conversion):
    if modifier == 'E':
        return conversion in _E_MODIFIERS
    return conversion in _O_MODIFIERS

def _offset_word(offset):
    # A zone offset spelled the way the C library does, sign and figures.
    sign = '+'
    if offset < 0:
        sign = '-'
        offset = -offset
    hours, left = divmod(offset, 3600)
    minutes = divmod(left, 60)[0]
    return sign + _pair(hours) + _pair(minutes)

def _case(conversion, text, flags):
    # A caret raises every conversion but the lower-case meridian; a
    # hash changes the case of the day and month names and the meridian
    # only, as the C library's own flag does.
    if '^' in flags and conversion != 'P':
        text = text.upper()
    if '#' in flags and conversion and conversion in 'aAbBhp':
        text = text.lower() if text.isupper() else text.upper()
    return text

def _field(conversion, text, flags, digits, natural, pad):
    # Lay the figures out the width the C library would: its own width
    # unless one is written, a flag choosing the filler, and the whole
    # shortened only when the conversion carries that few figures.
    wide = natural
    fill = pad
    for flag in flags:
        if flag == '-':
            fill = ' '
            wide = 0
        elif flag == '0':
            fill = '0'
            if wide == 0:
                wide = natural
        elif flag == '_':
            fill = ' '
    if digits:
        wanted = int(digits)
        if wanted > wide:
            wide = wanted
    # The zone name keeps its own shape however a negative flag meets a
    # width, the one conversion the C library lays out differently.
    if conversion == 'Z' and '-' in flags:
        wide = len(text)
    if wide > len(text):
        text = fill * (wide - len(text)) + text
    return _case(conversion, text, flags)

def strftime(format, fields=None):
    # Directive by directive the way the host's C library writes it in
    # the C locale: every conversion, the flags and field width written
    # before it, the E and O modifiers, and any conversion the library
    # would not know left as written.
    if fields is None:
        fields = localtime()
    parts = _tm_checked(fields)
    year, month, day, hour, minute, second, weekday, day_of_year, isdst = parts
    zone = getattr(fields, 'tm_zone', None)
    offset = getattr(fields, 'tm_gmtoff', None)
    # The C and POSIX locales stay with the writer below. Any other
    # LC_TIME is drawn by the host library, which knows that locale.
    active = _host_clock('locale', 2)
    if active and active != 'C' and active != 'POSIX':
        return _host_clock('picture', format, tuple(parts), zone, offset)
    iso = None
    out = ''
    at = 0
    span = len(format)
    while at < span:
        ch = format[at]
        at += 1
        if ch != '%':
            out += ch
            continue
        spec = at
        flags = ''
        while at < span and format[at] in '-_0^#':
            flags += format[at]
            at += 1
        digits = ''
        while at < span and format[at].isdigit():
            digits += format[at]
            at += 1
        if at < span and format[at] in 'EO':
            modifier = format[at]
            at += 1
        else:
            modifier = ''
        if at >= span:
            out += _field('', '%' + format[spec:], flags, digits, 0, ' ')
            break
        # A width written before the conversion, as the C library reads
        # it: the year spread to that many figures with leading zeros.
        ahead = at
        while ahead < len(format) and '0' <= format[ahead] <= '9':
            ahead += 1
        if ahead > at and ahead < len(format) and format[ahead] == 'Y':
            width = int(format[at:ahead])
            at = ahead + 1
            word = str(year)
            if len(word) < width:
                filling = '0' * (width - len(word))
                word = ('-' + filling + word[1:]) if word[0] == '-' else (filling + word)
            out += word
            continue
        ch = format[at]
        at += 1
        if modifier and not _modifier_fits(modifier, ch):
            out += _field('', '%' + format[spec:at], flags, digits, 0, ' ')
            continue
        if ch == 'a':
            out += _field('a', _DAY_SHORT[weekday], flags, digits, 0, ' ')
        elif ch == 'A':
            out += _field('A', _DAY_FULL[weekday], flags, digits, 0, ' ')
        elif ch == 'b' or ch == 'h':
            out += _field('b', _MONTH_SHORT[month], flags, digits, 0, ' ')
        elif ch == 'B':
            out += _field('B', _MONTH_FULL[month], flags, digits, 0, ' ')
        elif ch == 'c':
            out += _case('c', asctime(parts), flags)
        elif ch == 'C':
            out += _field('C', str(divmod(year, 100)[0]), flags, digits, 1, '0')
        elif ch == 'd':
            out += _field('d', str(day), flags, digits, 2, '0')
        elif ch == 'D' or ch == 'x':
            out += _field('D', _pair(month) + '/' + _pair(day) + '/' + _pair(divmod(year, 100)[1]), flags, digits, 0, ' ')
        elif ch == 'e':
            out += _field('e', str(day), flags, digits, 2, ' ')
        elif ch == 'F':
            out += _case('F', str(year) + '-' + _pair(month) + '-' + _pair(day), flags)
        elif ch == 'g':
            if iso is None:
                iso = _iso(year, weekday, day_of_year)
            out += _field('g', str(divmod(iso[0], 100)[1]), flags, digits, 2, '0')
        elif ch == 'G':
            if iso is None:
                iso = _iso(year, weekday, day_of_year)
            out += _field('G', str(iso[0]), flags, digits, 1, '0')
        elif ch == 'H':
            out += _field('H', str(hour), flags, digits, 2, '0')
        elif ch == 'I':
            twelve = divmod(hour, 12)[1]
            out += _field('I', str(12 if twelve == 0 else twelve), flags, digits, 2, '0')
        elif ch == 'j':
            out += _field('j', str(day_of_year), flags, digits, 3, '0')
        elif ch == 'k':
            out += _field('k', str(hour), flags, digits, 2, ' ')
        elif ch == 'l':
            twelve = divmod(hour, 12)[1]
            out += _field('l', str(12 if twelve == 0 else twelve), flags, digits, 2, ' ')
        elif ch == 'm':
            out += _field('m', str(month), flags, digits, 2, '0')
        elif ch == 'M':
            out += _field('M', str(minute), flags, digits, 2, '0')
        elif ch == 'n':
            out += _field('n', '\n', flags, digits, 0, ' ')
        elif ch == 'p':
            out += _field('p', 'AM' if hour < 12 else 'PM', flags, digits, 2, ' ')
        elif ch == 'P':
            out += _field('P', 'am' if hour < 12 else 'pm', flags, digits, 2, ' ')
        elif ch == 'r':
            twelve = divmod(hour, 12)[1]
            out += _case('r', _pair(12 if twelve == 0 else twelve) + ':' + _pair(minute) + ':' + _pair(second) + ' ' + ('AM' if hour < 12 else 'PM'), flags)
        elif ch == 'R':
            out += _field('R', _pair(hour) + ':' + _pair(minute), flags, digits, 0, ' ')
        elif ch == 's':
            days = _ymd2ord(year, month, day) - _EPOCH_ORD
            out += _field('s', str(days * 86400 + hour * 3600 + minute * 60 + second), flags, digits, 0, ' ')
        elif ch == 'S':
            out += _field('S', str(second), flags, digits, 2, '0')
        elif ch == 't':
            out += _field('t', '\t', flags, digits, 0, ' ')
        elif ch == 'T' or ch == 'X':
            out += _field('T', _pair(hour) + ':' + _pair(minute) + ':' + _pair(second), flags, digits, 0, ' ')
        elif ch == 'u':
            out += _field('u', str(weekday + 1), flags, digits, 1, '0')
        elif ch == 'U':
            sunday = divmod(weekday + 1, 7)[1]
            out += _field('U', str(divmod(day_of_year - 1 + 7 - sunday, 7)[0]), flags, digits, 2, '0')
        elif ch == 'V':
            if iso is None:
                iso = _iso(year, weekday, day_of_year)
            out += _field('V', str(iso[1]), flags, digits, 2, '0')
        elif ch == 'w':
            out += _field('w', str(divmod(weekday + 1, 7)[1]), flags, digits, 1, '0')
        elif ch == 'W':
            out += _field('W', str(divmod(day_of_year - 1 + 7 - weekday, 7)[0]), flags, digits, 2, '0')
        elif ch == 'y':
            out += _field('y', str(divmod(year, 100)[1]), flags, digits, 2, '0')
        elif ch == 'Y':
            out += _field('Y', str(year), flags, digits, 1, '0')
        elif ch == 'z':
            out += _field('z', '+0000' if offset is None else _offset_word(offset), flags, digits, 0, ' ')
        elif ch == 'Z':
            # A name on the tuple wins. Otherwise the process zone is
            # used when daylight is known, and nothing when it is not.
            if zone is not None:
                out += _field('Z', zone, flags, digits, 0, ' ')
            elif isdst > 0:
                out += _field('Z', tzname[1], flags, digits, 0, ' ')
            elif isdst == 0:
                out += _field('Z', tzname[0], flags, digits, 0, ' ')
        elif ch == '%':
            out += _field('%', '%', flags, digits, 0, ' ')
        else:
            out += _field('', '%' + format[spec:at], flags, digits, 0, ' ')
    return out

# The native struct_time keeps its two named non-sequence fields.
_STRUCT_TM_ITEMS = struct_time.n_fields

def strptime(string, format='%a %b %d %H:%M:%S %Y'):
    import _strptime
    return _strptime._strptime_time(string, format)
