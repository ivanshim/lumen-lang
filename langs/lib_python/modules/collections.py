# The small containers below keep their contents in ordinary arrays and
# maps. Where the object protocol is wanting, no silent stand-in is used.
def OrderedDict(items=None, **keywords):
    result = {}
    if items is not None:
        if type(items) == type({}):
            for key in list(items):
                result[key] = items[key]
        else:
            for key, value in items:
                result[key] = value
    for key in list(keywords):
        result[key] = keywords[key]
    return result

def namedtuple(typename, field_names, rename=False, defaults=None, module=None):
    if rename or defaults is not None:
        raise 'NotImplementedError: namedtuple renaming and defaults are not supported'
    if type(field_names) == type(''):
        names = []
        word = ''
        for letter in list(field_names + ' '):
            if letter == ' ' or letter == ',':
                if word != '':
                    names.append(word)
                    word = ''
            else:
                word += letter
    else:
        names = list(field_names)
    class Record:
        def __init__(self, *values):
            if len(values) != len(self._fields):
                raise 'TypeError: wrong number of namedtuple fields'
            for i in range(len(values)):
                setattr(self, self._fields[i], values[i])
    Record._fields = names
    Record.__name__ = typename
    return Record

class deque:
    def __init__(self, iterable=(), maxlen=None):
        self.data = list(iterable)
        self.maxlen = maxlen
        if maxlen is not None:
            if maxlen < 0:
                raise 'ValueError: maxlen must be non-negative'
            self.data = self.data[len(self.data) - maxlen:] if maxlen else []

    def append(self, value):
        self.data = self.data + [value]
        if self.maxlen is not None and len(self.data) > self.maxlen:
            self.data = self.data[1:]

    def appendleft(self, value):
        self.data = [value] + self.data
        if self.maxlen is not None and len(self.data) > self.maxlen:
            self.data = self.data[:-1]

    def pop(self):
        if len(self.data) == 0:
            raise 'IndexError: pop from an empty deque'
        value = self.data[-1]
        self.data = self.data[:-1]
        return value

    def popleft(self):
        if len(self.data) == 0:
            raise 'IndexError: pop from an empty deque'
        value = self.data[0]
        self.data = self.data[1:]
        return value

    def extend(self, values):
        for value in values:
            self.append(value)

    def extendleft(self, values):
        for value in values:
            self.appendleft(value)

    def clear(self):
        self.data = []

    def rotate(self, n=1):
        if len(self.data) != 0:
            n %= len(self.data)
            self.data = self.data[-n:] + self.data[:-n]

    def count(self, value):
        return sum([1 for held in self.data if held == value])

class defaultdict:
    def __init__(self, default_factory=None, *args, **kwargs):
        raise 'NotImplementedError: defaultdict needs object indexing methods'

class Counter:
    def __init__(self, iterable=None, **kwargs):
        raise 'NotImplementedError: Counter needs object indexing methods'
