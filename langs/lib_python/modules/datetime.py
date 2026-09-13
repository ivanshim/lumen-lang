# Dates and times of day, counted the way the reference counts them.
#
# The calendar arithmetic here is the proleptic Gregorian one CPython
# uses, so a day number, a weekday and the length of a month come out
# the same. Three things differ and are worth saying plainly. A date is
# built through __init__ rather than __new__, so a program that reaches
# for date.__new__ finds nothing to call. The clock the host offers
# counts seconds from the epoch and says nothing about the zone the host
# stands in, so now() and today() answer in UTC and local time is taken
# to be UTC; a program that needs the real local offset does not get it
# here. And strftime is written out below directive by directive in the
# C locale, rather than handed to the host, so the day and month names
# are English whatever the host is set to.
import time as _time

__all__ = ['MINYEAR', 'MAXYEAR', 'date', 'datetime', 'time', 'timedelta',
           'timezone', 'tzinfo', 'UTC']

MINYEAR = 1
MAXYEAR = 9999

_DAYS_IN_MONTH = [-1, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
_DAYS_BEFORE_MONTH = [-1, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]

_DAY_NAMES = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday',
              'Saturday', 'Sunday']
_MONTH_NAMES = ['', 'January', 'February', 'March', 'April', 'May', 'June',
                'July', 'August', 'September', 'October', 'November',
                'December']

# The number of days in four hundred years, in a hundred, and in four.
_DI400Y = 146097
_DI100Y = 36524
_DI4Y = 1461


def _is_leap(year):
    return year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)


def _days_before_year(year):
    past = year - 1
    return past * 365 + past // 4 - past // 100 + past // 400


def _days_in_month(year, month):
    if month == 2 and _is_leap(year):
        return 29
    return _DAYS_IN_MONTH[month]


def _days_before_month(year, month):
    days = _DAYS_BEFORE_MONTH[month]
    if month > 2 and _is_leap(year):
        days += 1
    return days


def _ymd2ord(year, month, day):
    return _days_before_year(year) + _days_before_month(year, month) + day


def _ord2ymd(n):
    # Count off whole four-hundred-year, hundred-year and four-year
    # spans, then the years left over, and what remains is the day of
    # the year.
    n -= 1
    n400, n = divmod(n, _DI400Y)
    year = n400 * 400 + 1
    n100, n = divmod(n, _DI100Y)
    n4, n = divmod(n, _DI4Y)
    n1, n = divmod(n, 365)
    year += n100 * 100 + n4 * 4 + n1
    if n1 == 4 or n100 == 4:
        return (year - 1, 12, 31)
    leap = n1 == 3 and (n4 != 24 or n100 == 3)
    month = (n + 50) >> 5
    before = _DAYS_BEFORE_MONTH[month]
    if month > 2 and leap:
        before += 1
    if before > n:
        month -= 1
        before = _DAYS_BEFORE_MONTH[month]
        if month > 2 and leap:
            before += 1
    n -= before
    return (year, month, n + 1)


def _checked_date(year, month, day):
    if year < MINYEAR or year > MAXYEAR:
        raise 'ValueError: year is out of range'
    if month < 1 or month > 12:
        raise 'ValueError: month must be in 1..12'
    if day < 1 or day > _days_in_month(year, month):
        raise 'ValueError: day is out of range for month'


def _checked_time(hour, minute, second, microsecond):
    if hour < 0 or hour > 23:
        raise 'ValueError: hour must be in 0..23'
    if minute < 0 or minute > 59:
        raise 'ValueError: minute must be in 0..59'
    if second < 0 or second > 59:
        raise 'ValueError: second must be in 0..59'
    if microsecond < 0 or microsecond > 999999:
        raise 'ValueError: microsecond must be in 0..999999'


def _padded(number, width):
    text = str(number)
    while len(text) < width:
        text = '0' + text
    return text


def _offset_text(delta, colon):
    # The +HHMM an offset from UTC is written as.
    if delta is None:
        return ''
    whole = delta.days * 86400 + delta.seconds
    sign = '+'
    if whole < 0:
        sign = '-'
        whole = -whole
    hours, rest = divmod(whole // 60, 60)
    if colon:
        return sign + _padded(hours, 2) + ':' + _padded(rest, 2)
    return sign + _padded(hours, 2) + _padded(rest, 2)


def _strftime(spec, year, month, day, hour, minute, second, microsecond,
              offset, zone_name):
    # The C locale, written out. A directive this runtime does not know
    # is left as it stands, which is what CPython's own fallback does on
    # a host that does not know it either.
    ordinal = _ymd2ord(year, month, day)
    weekday = (ordinal + 6) % 7
    yearday = ordinal - _days_before_year(year)
    out = []
    at = 0
    while at < len(spec):
        letter = spec[at]
        if letter != '%' or at + 1 >= len(spec):
            out.append(letter)
            at += 1
            continue
        code = spec[at + 1]
        at += 2
        if code == 'Y':
            out.append(_padded(year, 4))
        elif code == 'y':
            out.append(_padded(year % 100, 2))
        elif code == 'm':
            out.append(_padded(month, 2))
        elif code == 'd':
            out.append(_padded(day, 2))
        elif code == 'H':
            out.append(_padded(hour, 2))
        elif code == 'I':
            shown = hour % 12
            if shown == 0:
                shown = 12
            out.append(_padded(shown, 2))
        elif code == 'M':
            out.append(_padded(minute, 2))
        elif code == 'S':
            out.append(_padded(second, 2))
        elif code == 'f':
            out.append(_padded(microsecond, 6))
        elif code == 'p':
            if hour < 12:
                out.append('AM')
            else:
                out.append('PM')
        elif code == 'a':
            out.append(_DAY_NAMES[weekday][:3])
        elif code == 'A':
            out.append(_DAY_NAMES[weekday])
        elif code == 'b':
            out.append(_MONTH_NAMES[month][:3])
        elif code == 'B':
            out.append(_MONTH_NAMES[month])
        elif code == 'j':
            out.append(_padded(yearday, 3))
        elif code == 'w':
            out.append(str((weekday + 1) % 7))
        elif code == 'u':
            out.append(str(weekday + 1))
        elif code == 'U':
            out.append(_padded((yearday + 6 - ((weekday + 1) % 7)) // 7, 2))
        elif code == 'W':
            out.append(_padded((yearday + 6 - weekday) // 7, 2))
        elif code == 'z':
            out.append(_offset_text(offset, False))
        elif code == 'Z':
            if zone_name is None:
                out.append('')
            else:
                out.append(zone_name)
        elif code == 'c':
            shown = str(day)
            if len(shown) < 2:
                shown = ' ' + shown
            out.append(_DAY_NAMES[weekday][:3] + ' ' + _MONTH_NAMES[month][:3]
                       + ' ' + shown + ' ' + _padded(hour, 2) + ':'
                       + _padded(minute, 2) + ':' + _padded(second, 2) + ' '
                       + _padded(year, 4))
        elif code == 'x':
            out.append(_padded(month, 2) + '/' + _padded(day, 2) + '/'
                       + _padded(year % 100, 2))
        elif code == 'X':
            out.append(_padded(hour, 2) + ':' + _padded(minute, 2) + ':'
                       + _padded(second, 2))
        elif code == '%':
            out.append('%')
        else:
            out.append('%' + code)
    return ''.join(out)


class timedelta:
    def __init__(self, days=0, seconds=0, microseconds=0, milliseconds=0,
                 minutes=0, hours=0, weeks=0):
        whole = (((weeks * 7 + days) * 24 + hours) * 60 + minutes) * 60 + seconds
        tiny = milliseconds * 1000 + microseconds
        carried, tiny = divmod(tiny, 1000000)
        whole += carried
        carried, whole = divmod(whole, 86400)
        self._days = carried
        self._seconds = whole
        self._microseconds = tiny
        if self._days < -999999999 or self._days > 999999999:
            raise 'OverflowError: timedelta is out of range'

    @property
    def days(self):
        return self._days

    @property
    def seconds(self):
        return self._seconds

    @property
    def microseconds(self):
        return self._microseconds

    def total_seconds(self):
        return (self._days * 86400 + self._seconds) + self._microseconds / 1000000

    def _whole(self):
        return (self._days * 86400 + self._seconds) * 1000000 + self._microseconds

    def __repr__(self):
        return 'datetime.timedelta(days=' + str(self._days) + ', seconds=' \
            + str(self._seconds) + ', microseconds=' + str(self._microseconds) + ')'

    def __str__(self):
        minutes, seconds = divmod(self._seconds, 60)
        hours, minutes = divmod(minutes, 60)
        text = str(hours) + ':' + _padded(minutes, 2) + ':' + _padded(seconds, 2)
        if self._days:
            word = ' day, '
            if self._days != 1 and self._days != -1:
                word = ' days, '
            text = str(self._days) + word + text
        if self._microseconds:
            text = text + '.' + _padded(self._microseconds, 6)
        return text

    def __add__(self, other):
        if not isinstance(other, timedelta):
            raise 'TypeError: a timedelta is required'
        return timedelta(0, 0, self._whole() + other._whole())

    def __radd__(self, other):
        return self.__add__(other)

    def __sub__(self, other):
        if not isinstance(other, timedelta):
            raise 'TypeError: a timedelta is required'
        return timedelta(0, 0, self._whole() - other._whole())

    def __neg__(self):
        return timedelta(0, 0, -self._whole())

    def __abs__(self):
        if self._days < 0:
            return -self
        return self

    def __mul__(self, other):
        return timedelta(0, 0, self._whole() * other)

    def __rmul__(self, other):
        return self.__mul__(other)

    def __bool__(self):
        return self._whole() != 0

    def __eq__(self, other):
        if not isinstance(other, timedelta):
            return False
        return self._whole() == other._whole()

    def __ne__(self, other):
        return not self.__eq__(other)

    def __lt__(self, other):
        return self._whole() < other._whole()

    def __le__(self, other):
        return self._whole() <= other._whole()

    def __gt__(self, other):
        return self._whole() > other._whole()

    def __ge__(self, other):
        return self._whole() >= other._whole()

    def __hash__(self):
        return hash(self._whole())


class tzinfo:
    def utcoffset(self, dt):
        raise 'NotImplementedError: tzinfo.utcoffset must be given by a subclass'

    def dst(self, dt):
        raise 'NotImplementedError: tzinfo.dst must be given by a subclass'

    def tzname(self, dt):
        raise 'NotImplementedError: tzinfo.tzname must be given by a subclass'


class timezone(tzinfo):
    def __init__(self, offset, name=None):
        if not isinstance(offset, timedelta):
            raise 'TypeError: a timedelta is required'
        self._offset = offset
        self._name = name

    def utcoffset(self, dt):
        return self._offset

    def dst(self, dt):
        return None

    def tzname(self, dt):
        if self._name is not None:
            return self._name
        if self._offset == timedelta(0):
            return 'UTC'
        return 'UTC' + _offset_text(self._offset, True)

    def __eq__(self, other):
        if not isinstance(other, timezone):
            return False
        return self._offset == other._offset

    def __hash__(self):
        return hash(self._offset)

    def __repr__(self):
        return 'datetime.timezone(' + repr(self._offset) + ')'

    def __str__(self):
        return self.tzname(None)


UTC = timezone(timedelta(0), 'UTC')


class date:
    def __init__(self, year, month, day):
        _checked_date(year, month, day)
        self._year = year
        self._month = month
        self._day = day

    @property
    def year(self):
        return self._year

    @property
    def month(self):
        return self._month

    @property
    def day(self):
        return self._day

    @classmethod
    def fromordinal(cls, n):
        year, month, day = _ord2ymd(n)
        return cls(year, month, day)

    @classmethod
    def fromtimestamp(cls, stamp):
        whole = int(stamp // 86400)
        year, month, day = _ord2ymd(whole + 719163)
        return cls(year, month, day)

    @classmethod
    def today(cls):
        return cls.fromtimestamp(_time.time())

    @classmethod
    def fromisoformat(cls, text):
        if len(text) != 10 or text[4] != '-' or text[7] != '-':
            raise 'ValueError: a date in the form YYYY-MM-DD is required'
        return cls(int(text[0:4]), int(text[5:7]), int(text[8:10]))

    def toordinal(self):
        return _ymd2ord(self._year, self._month, self._day)

    def weekday(self):
        return (self.toordinal() + 6) % 7

    def isoweekday(self):
        return self.toordinal() % 7 or 7

    def isoformat(self):
        return _padded(self._year, 4) + '-' + _padded(self._month, 2) + '-' \
            + _padded(self._day, 2)

    def ctime(self):
        return self.strftime('%c')

    def replace(self, year=None, month=None, day=None):
        if year is None:
            year = self._year
        if month is None:
            month = self._month
        if day is None:
            day = self._day
        return type(self)(year, month, day)

    def timetuple(self):
        raise 'NotImplementedError: date.timetuple needs the struct_time of the time module, which is not here'

    def strftime(self, spec):
        return _strftime(spec, self._year, self._month, self._day, 0, 0, 0, 0,
                         None, None)

    def __format__(self, spec):
        if spec == '':
            return str(self)
        return self.strftime(spec)

    def __str__(self):
        return self.isoformat()

    def __repr__(self):
        return 'datetime.date(' + str(self._year) + ', ' + str(self._month) \
            + ', ' + str(self._day) + ')'

    def _key(self):
        return (self._year, self._month, self._day)

    def __eq__(self, other):
        if not isinstance(other, date):
            return False
        return self._key() == other._key()

    def __ne__(self, other):
        return not self.__eq__(other)

    def __lt__(self, other):
        return self._key() < other._key()

    def __le__(self, other):
        return self._key() <= other._key()

    def __gt__(self, other):
        return self._key() > other._key()

    def __ge__(self, other):
        return self._key() >= other._key()

    def __hash__(self):
        return hash(self._key())

    def __add__(self, other):
        if not isinstance(other, timedelta):
            raise 'TypeError: a timedelta is required'
        return type(self).fromordinal(self.toordinal() + other.days)

    def __radd__(self, other):
        return self.__add__(other)

    def __sub__(self, other):
        if isinstance(other, timedelta):
            return type(self).fromordinal(self.toordinal() - other.days)
        if isinstance(other, date):
            return timedelta(self.toordinal() - other.toordinal())
        raise 'TypeError: a date or a timedelta is required'


class time:
    def __init__(self, hour=0, minute=0, second=0, microsecond=0, tzinfo=None):
        _checked_time(hour, minute, second, microsecond)
        self._hour = hour
        self._minute = minute
        self._second = second
        self._microsecond = microsecond
        self._tzinfo = tzinfo

    @property
    def hour(self):
        return self._hour

    @property
    def minute(self):
        return self._minute

    @property
    def second(self):
        return self._second

    @property
    def microsecond(self):
        return self._microsecond

    @property
    def tzinfo(self):
        return self._tzinfo

    def utcoffset(self):
        if self._tzinfo is None:
            return None
        return self._tzinfo.utcoffset(None)

    def tzname(self):
        if self._tzinfo is None:
            return None
        return self._tzinfo.tzname(None)

    def isoformat(self):
        text = _padded(self._hour, 2) + ':' + _padded(self._minute, 2) + ':' \
            + _padded(self._second, 2)
        if self._microsecond:
            text += '.' + _padded(self._microsecond, 6)
        offset = self.utcoffset()
        if offset is not None:
            text += _offset_text(offset, True)
        return text

    def strftime(self, spec):
        return _strftime(spec, 1900, 1, 1, self._hour, self._minute,
                         self._second, self._microsecond, self.utcoffset(),
                         self.tzname())

    def __format__(self, spec):
        if spec == '':
            return str(self)
        return self.strftime(spec)

    def __str__(self):
        return self.isoformat()

    def __repr__(self):
        return 'datetime.time(' + str(self._hour) + ', ' + str(self._minute) \
            + ', ' + str(self._second) + ', ' + str(self._microsecond) + ')'

    def _key(self):
        return (self._hour, self._minute, self._second, self._microsecond)

    def __eq__(self, other):
        if not isinstance(other, time):
            return False
        return self._key() == other._key()

    def __ne__(self, other):
        return not self.__eq__(other)

    def __lt__(self, other):
        return self._key() < other._key()

    def __le__(self, other):
        return self._key() <= other._key()

    def __gt__(self, other):
        return self._key() > other._key()

    def __ge__(self, other):
        return self._key() >= other._key()

    def __hash__(self):
        return hash(self._key())


class datetime(date):
    def __init__(self, year, month, day, hour=0, minute=0, second=0,
                 microsecond=0, tzinfo=None):
        _checked_date(year, month, day)
        _checked_time(hour, minute, second, microsecond)
        self._year = year
        self._month = month
        self._day = day
        self._hour = hour
        self._minute = minute
        self._second = second
        self._microsecond = microsecond
        self._tzinfo = tzinfo

    @property
    def hour(self):
        return self._hour

    @property
    def minute(self):
        return self._minute

    @property
    def second(self):
        return self._second

    @property
    def microsecond(self):
        return self._microsecond

    @property
    def tzinfo(self):
        return self._tzinfo

    @classmethod
    def fromtimestamp(cls, stamp, tz=None):
        whole = int(stamp // 1)
        tiny = int(round((stamp - whole) * 1000000))
        if tiny >= 1000000:
            whole += 1
            tiny -= 1000000
        days, rest = divmod(whole, 86400)
        year, month, day = _ord2ymd(days + 719163)
        minutes, second = divmod(rest, 60)
        hour, minute = divmod(minutes, 60)
        return cls(year, month, day, hour, minute, second, tiny, tz)

    @classmethod
    def utcfromtimestamp(cls, stamp):
        return cls.fromtimestamp(stamp)

    @classmethod
    def now(cls, tz=None):
        # The host clock counts seconds from the epoch and says nothing
        # about the zone, so this is UTC under whatever name is asked
        # for.
        return cls.fromtimestamp(_time.time(), tz)

    @classmethod
    def utcnow(cls):
        return cls.fromtimestamp(_time.time())

    @classmethod
    def today(cls):
        return cls.now()

    @classmethod
    def combine(cls, day, moment, tzinfo=True):
        zone = moment.tzinfo
        if tzinfo is not True:
            zone = tzinfo
        return cls(day.year, day.month, day.day, moment.hour, moment.minute,
                   moment.second, moment.microsecond, zone)

    @classmethod
    def fromordinal(cls, n):
        year, month, day = _ord2ymd(n)
        return cls(year, month, day)

    def date(self):
        return date(self._year, self._month, self._day)

    def time(self):
        return time(self._hour, self._minute, self._second, self._microsecond)

    def timetz(self):
        return time(self._hour, self._minute, self._second, self._microsecond,
                    self._tzinfo)

    def utcoffset(self):
        if self._tzinfo is None:
            return None
        return self._tzinfo.utcoffset(self)

    def dst(self):
        if self._tzinfo is None:
            return None
        return self._tzinfo.dst(self)

    def tzname(self):
        if self._tzinfo is None:
            return None
        return self._tzinfo.tzname(self)

    def timestamp(self):
        whole = (self.toordinal() - 719163) * 86400 + self._hour * 3600 \
            + self._minute * 60 + self._second
        offset = self.utcoffset()
        if offset is not None:
            whole -= offset.days * 86400 + offset.seconds
        return whole + self._microsecond / 1000000

    def replace(self, year=None, month=None, day=None, hour=None, minute=None,
                second=None, microsecond=None, tzinfo=True):
        if year is None:
            year = self._year
        if month is None:
            month = self._month
        if day is None:
            day = self._day
        if hour is None:
            hour = self._hour
        if minute is None:
            minute = self._minute
        if second is None:
            second = self._second
        if microsecond is None:
            microsecond = self._microsecond
        zone = self._tzinfo
        if tzinfo is not True:
            zone = tzinfo
        return type(self)(year, month, day, hour, minute, second, microsecond,
                          zone)

    def isoformat(self, sep='T'):
        text = _padded(self._year, 4) + '-' + _padded(self._month, 2) + '-' \
            + _padded(self._day, 2) + sep + _padded(self._hour, 2) + ':' \
            + _padded(self._minute, 2) + ':' + _padded(self._second, 2)
        if self._microsecond:
            text += '.' + _padded(self._microsecond, 6)
        offset = self.utcoffset()
        if offset is not None:
            text += _offset_text(offset, True)
        return text

    def strftime(self, spec):
        return _strftime(spec, self._year, self._month, self._day, self._hour,
                         self._minute, self._second, self._microsecond,
                         self.utcoffset(), self.tzname())

    def ctime(self):
        return self.strftime('%c')

    def __str__(self):
        return self.isoformat(' ')

    def __repr__(self):
        return 'datetime.datetime(' + str(self._year) + ', ' + str(self._month) \
            + ', ' + str(self._day) + ', ' + str(self._hour) + ', ' \
            + str(self._minute) + ', ' + str(self._second) + ')'

    def _key(self):
        return (self._year, self._month, self._day, self._hour, self._minute,
                self._second, self._microsecond)

    def _stamp(self):
        return ((self.toordinal() * 24 + self._hour) * 60 + self._minute) \
            * 60000000 + self._second * 1000000 + self._microsecond

    def __add__(self, other):
        if not isinstance(other, timedelta):
            raise 'TypeError: a timedelta is required'
        return datetime.fromordinal(1)._made(self._stamp() + other._whole(),
                                             self._tzinfo)

    def __radd__(self, other):
        return self.__add__(other)

    def __sub__(self, other):
        if isinstance(other, timedelta):
            return self.__add__(-other)
        if isinstance(other, datetime):
            return timedelta(0, 0, self._stamp() - other._stamp())
        raise 'TypeError: a datetime or a timedelta is required'

    def _made(self, stamp, zone):
        days, rest = divmod(stamp, 86400000000)
        year, month, day = _ord2ymd(days)
        rest, tiny = divmod(rest, 1000000)
        minutes, second = divmod(rest, 60)
        hour, minute = divmod(minutes, 60)
        return datetime(year, month, day, hour, minute, second, tiny, zone)


date.min = date(1, 1, 1)
date.max = date(9999, 12, 31)
date.resolution = timedelta(1)
time.min = time(0, 0, 0)
time.max = time(23, 59, 59, 999999)
time.resolution = timedelta(0, 0, 1)
datetime.min = datetime(1, 1, 1)
datetime.max = datetime(9999, 12, 31, 23, 59, 59, 999999)
datetime.resolution = timedelta(0, 0, 1)
timedelta.min = timedelta(-999999999)
timedelta.max = timedelta(999999999, 86399, 999999)
timedelta.resolution = timedelta(0, 0, 1)
timezone.utc = UTC
