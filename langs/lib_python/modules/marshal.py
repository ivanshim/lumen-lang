# A value handed out as bytes and taken back as itself. The writing is
# this runtime's own and not the reference implementation's marshal
# format, so what is written here is read back only here; it says so
# plainly rather than pretending to a format it does not have. What it
# does keep is what marshal promises: the built-in kinds and nothing of
# a class of its own, the two singletons back as the singletons, and a
# thing that stood in two places standing in two places again.
#
# The writing itself is lent to pickle, which has the same work to do
# and takes a slice as well; see _write and _read at the foot of the
# file.

version = 4


class _Writer:
    def __init__(self, slices, refuse):
        self.pieces = []
        self.memo = {}
        self.slices = slices
        self.refuse = refuse

    def text(self):
        return ''.join(self.pieces)

    def held(self, value):
        # A thing already written stands again as the number of its
        # place. Every container takes a place, and takes it before its
        # contents, so that a thing holding itself is read back.
        where = id(value)
        if where in self.memo:
            self.pieces.append('b' + str(self.memo[where]) + ';')
            return True
        self.memo[where] = len(self.memo)
        return False

    def put(self, value):
        if value is None:
            self.pieces.append('n')
            return
        if value is True:
            self.pieces.append('T')
            return
        if value is False:
            self.pieces.append('F')
            return
        if isinstance(value, int):
            self.exactly(value, type(value) is int)
            self.pieces.append('i' + str(value) + ';')
            return
        if isinstance(value, float):
            self.exactly(value, type(value) is float)
            self.pieces.append('r' + repr(value) + ';')
            return
        if isinstance(value, complex):
            self.exactly(value, type(value) is complex)
            self.pieces.append('x' + repr(value.real) + ';' + repr(value.imag) + ';')
            return
        if isinstance(value, str):
            self.exactly(value, type(value) is str)
            self.pieces.append('u' + str(len(value)) + ';' + value)
            return
        if isinstance(value, bytes):
            self.exactly(value, type(value) is bytes)
            self.pieces.append('y' + str(len(value)) + ';' + value.hex())
            return
        if isinstance(value, tuple):
            self.exactly(value, type(value) is tuple)
            if self.held(value):
                return
            self.pieces.append('P' + str(len(value)) + ';')
            for item in value:
                self.put(item)
            return
        if isinstance(value, list):
            self.exactly(value, type(value) is list)
            if self.held(value):
                return
            self.pieces.append('L' + str(len(value)) + ';')
            for item in value:
                self.put(item)
            return
        if isinstance(value, dict):
            self.exactly(value, type(value) is dict)
            if self.held(value):
                return
            self.pieces.append('D' + str(len(value)) + ';')
            for key in value:
                self.put(key)
                self.put(value[key])
            return
        if isinstance(value, set):
            # This runtime holds a set and a frozen set as one kind and
            # gives no way to tell them apart, so both are written as a
            # set and a frozen set comes back thawed. It compares equal
            # to what went in; it is not the same kind.
            self.exactly(value, type(value) is set or type(value) is frozenset)
            if self.held(value):
                return
            self.pieces.append('S' + str(len(value)) + ';')
            for item in value:
                self.put(item)
            return
        parts = _slice_parts(value)
        if parts is not None and self.slices:
            # A slice takes no place among the things held: it cannot
            # hold itself, and this runtime gives a slice no id to keep
            # a place by, so one standing twice is written twice.
            self.pieces.append('Q')
            self.put(parts[0])
            self.put(parts[1])
            self.put(parts[2])
            return
        self.refuse(value)

    def exactly(self, value, plain):
        # A class of its own built on a built-in kind is refused rather
        # than written as the kind beneath it, which would hand back
        # something of the wrong class.
        if not plain:
            self.refuse(value)


class _Reader:
    def __init__(self, text):
        self.text = text
        self.at = 0
        self.memo = []

    def word(self):
        start = self.at
        while self.at < len(self.text) and self.text[self.at] != ';':
            self.at += 1
        if self.at >= len(self.text):
            raise ValueError('bad marshal data')
        piece = self.text[start:self.at]
        self.at += 1
        return piece

    def count(self):
        return int(self.word())

    def characters(self, size):
        if self.at + size > len(self.text):
            raise ValueError('bad marshal data')
        piece = self.text[self.at:self.at + size]
        self.at += size
        return piece

    def reserve(self):
        where = len(self.memo)
        self.memo.append(None)
        return where

    def items(self, size):
        made = []
        done = 0
        while done < size:
            made.append(self.get())
            done += 1
        return made

    def get(self):
        if self.at >= len(self.text):
            raise ValueError('bad marshal data')
        tag = self.text[self.at]
        self.at += 1
        if tag == 'n':
            return None
        if tag == 'T':
            return True
        if tag == 'F':
            return False
        if tag == 'i':
            return int(self.word())
        if tag == 'r':
            return float(self.word())
        if tag == 'x':
            real = float(self.word())
            imaginary = float(self.word())
            return complex(real, imaginary)
        if tag == 'u':
            return self.characters(self.count())
        if tag == 'y':
            size = self.count()
            return bytes.fromhex(self.characters(size * 2))
        if tag == 'b':
            where = self.count()
            if where >= len(self.memo):
                raise ValueError('bad marshal data')
            standing = self.memo[where]
            if standing is None:
                raise ValueError('a loop through a tuple or a set cannot be read back')
            return standing
        if tag == 'L':
            size = self.count()
            made = []
            self.memo.append(made)
            done = 0
            while done < size:
                made.append(self.get())
                done += 1
            return made
        if tag == 'D':
            size = self.count()
            made = {}
            self.memo.append(made)
            done = 0
            while done < size:
                key = self.get()
                made[key] = self.get()
                done += 1
            return made
        if tag == 'P':
            size = self.count()
            where = self.reserve()
            made = tuple(self.items(size))
            self.memo[where] = made
            return made
        if tag == 'S':
            size = self.count()
            where = self.reserve()
            made = set(self.items(size))
            self.memo[where] = made
            return made
        if tag == 'Q':
            start = self.get()
            stop = self.get()
            step = self.get()
            return slice(start, stop, step)
        raise ValueError('bad marshal data')


def _slice_parts(value):
    # A slice is the one shape here that type() cannot name, so it is
    # asked for its own parts instead. Anything that will not answer for
    # all four is not a slice, whatever it says in refusing.
    try:
        start = value.start
        stop = value.stop
        step = value.step
        cut = value.indices
    except Exception:
        return None
    if not callable(cut):
        return None
    return (start, stop, step)


def _write(value, slices, refuse):
    writer = _Writer(slices, refuse)
    writer.put(value)
    return writer.text().encode()


def _read(data):
    if isinstance(data, str):
        raise TypeError('a str is not written-out bytes')
    return _Reader(data.decode()).get()


def _unwritable(value):
    raise ValueError('unmarshallable object')


def dumps(value, version=version):
    return _write(value, False, _unwritable)


def loads(data):
    return _read(data)


def dump(value, file, version=version):
    raise 'NotImplementedError: marshal.dump needs a file this runtime cannot write'


def load(file):
    raise 'NotImplementedError: marshal.load needs a file this runtime cannot read'


def __getattr__(name):
    raise 'NotImplementedError: marshal.' + name + ' is not supported'
