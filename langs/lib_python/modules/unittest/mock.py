# A stand-in put in an attribute's place while a test runs, and taken
# out again afterwards. Only the patching a test here asks for is
# written: an attribute of a thing already in hand, replaced by a stand-in
# that records its calls and answers what it was told to answer. The
# wider library -- specs, autospeccing, patching by dotted name, the
# magic methods -- refuses by name rather than standing in for itself.

class _Sentinel:
    def __init__(self, word):
        self.word = word

    def __repr__(self):
        return 'sentinel.' + self.word

DEFAULT = _Sentinel('DEFAULT')


class Mock:
    def __init__(self, name=None, return_value=DEFAULT, side_effect=None, wraps=None):
        self._mock_name = name
        self._mock_wraps = wraps
        self.return_value = return_value
        self._side_effect = None
        self.side_effect = side_effect
        self.called = False
        self.call_count = 0
        self.call_args = None
        self.call_args_list = []

    @property
    def side_effect(self):
        return self._side_effect

    @side_effect.setter
    def side_effect(self, value):
        # A fault, or a call, stands as it is given. Anything else that
        # can be walked stands for one answer per call, drawn in order;
        # what can neither be called nor walked stands as it is given,
        # to complain when it is drawn on.
        if (value is None or isinstance(value, BaseException)
                or isinstance(value, type) or callable(value)):
            self._side_effect = value
            return
        try:
            self._side_effect = iter(value)
        except TypeError:
            self._side_effect = value

    def __repr__(self):
        if self._mock_name is None:
            return '<Mock>'
        return '<Mock ' + str(self._mock_name) + '>'

    def __call__(self, *args, **kwargs):
        self.called = True
        self.call_count += 1
        self.call_args = (list(args), kwargs)
        self.call_args_list.append(self.call_args)
        effect = self.side_effect
        if effect is not None:
            if isinstance(effect, BaseException):
                raise effect
            if isinstance(effect, type):
                # A class named as the effect stands for what it makes:
                # a fault is raised, anything else is the answer.
                made = effect()
                if isinstance(made, BaseException):
                    raise made
                return made
            if callable(effect):
                answer = effect(*args, **kwargs)
                if answer is not DEFAULT:
                    return answer
            else:
                # One answer per call, drawn from the row given; the
                # row spent, there is nothing left to answer with.
                answer = next(effect)
                if isinstance(answer, BaseException):
                    raise answer
                return answer
        if self.return_value is DEFAULT:
            if self._mock_wraps is not None:
                return self._mock_wraps(*args, **kwargs)
            self.return_value = Mock(name='()')
        return self.return_value

    def reset_mock(self):
        self.called = False
        self.call_count = 0
        self.call_args = None
        self.call_args_list = []

    def assert_called(self):
        if not self.called:
            raise AssertionError('expected a call and there was none')

    def assert_not_called(self):
        if self.called:
            raise AssertionError('expected no call and there were ' + str(self.call_count))

    def assert_called_once(self):
        if self.call_count != 1:
            raise AssertionError('expected one call and there were ' + str(self.call_count))

    def assert_called_with(self, *args, **kwargs):
        if not self.called:
            raise AssertionError('expected a call and there was none')
        wanted = (list(args), kwargs)
        if self.call_args != wanted:
            raise AssertionError('expected ' + repr(wanted) + ' and the last call was ' + repr(self.call_args))

    def assert_called_once_with(self, *args, **kwargs):
        self.assert_called_once()
        self.assert_called_with(*args, **kwargs)

    def assert_any_call(self, *args, **kwargs):
        wanted = (list(args), kwargs)
        for made in self.call_args_list:
            if made == wanted:
                return
        raise AssertionError('expected ' + repr(wanted) + ' among the calls and it is not there')

    def __getattr__(self, name):
        # A name asked of a stand-in is a stand-in of its own, as the
        # reference library has it. A name that begins with an
        # underscore is the runtime's own business and is not answered
        # for, so what looks inside a mock sees what is really there.
        if name[:1] == '_':
            raise AttributeError(name)
        child = Mock(name=name)
        setattr(self, name, child)
        return child


# What the reference hands back from each of the double-underscore names
# a MagicMock answers to, when nothing has said otherwise.
_MAGIC_ANSWERS = {
    '__lt__': NotImplemented,
    '__gt__': NotImplemented,
    '__le__': NotImplemented,
    '__ge__': NotImplemented,
    '__int__': 1,
    '__index__': 1,
    '__float__': 1.0,
    '__complex__': 1j,
    '__bool__': True,
    '__len__': 0,
    '__contains__': False,
    '__exit__': False,
}


class MagicMock(Mock):
    # A stand-in that also answers the operations the reader reaches
    # through double-underscore names. Each of those is a stand-in of
    # its own, made once when the MagicMock is and kept on it under its
    # own name, so a test can count the calls to it and put a different
    # answer on it. The reader looks a double-underscore name up on the
    # class and never on the thing itself, so the methods below stand on
    # the class and pass the work to the stand-in kept beside them.
    #
    # Only the names this reader asks an object for are here. The
    # arithmetic ones, the asynchronous ones and the rest the reference
    # carries are not, so a MagicMock added to a number is refused
    # rather than answering with a stand-in.
    def __init__(self, name=None, return_value=DEFAULT, side_effect=None, wraps=None):
        self._magic = {}
        Mock.__init__(self, name, return_value, side_effect, wraps)
        for word in _MAGIC_ANSWERS:
            child = Mock(name=word)
            child.return_value = _MAGIC_ANSWERS[word]
            self._magic[word] = child
            setattr(self, word, child)
        walker = Mock(name='__iter__')
        walker.return_value = iter([])
        self._magic['__iter__'] = walker
        setattr(self, '__iter__', walker)

    def __repr__(self):
        if self._mock_name is None:
            return '<MagicMock>'
        return '<MagicMock ' + str(self._mock_name) + '>'

    def __lt__(self, other):
        return self._magic['__lt__'](other)

    def __gt__(self, other):
        return self._magic['__gt__'](other)

    def __le__(self, other):
        return self._magic['__le__'](other)

    def __ge__(self, other):
        return self._magic['__ge__'](other)

    def __int__(self):
        return self._magic['__int__']()

    def __index__(self):
        return self._magic['__index__']()

    def __float__(self):
        return self._magic['__float__']()

    def __complex__(self):
        return self._magic['__complex__']()

    def __bool__(self):
        return self._magic['__bool__']()

    def __len__(self):
        return self._magic['__len__']()

    def __contains__(self, item):
        return self._magic['__contains__'](item)

    def __iter__(self):
        return self._magic['__iter__']()

    def __enter__(self):
        return MagicMock(name='__enter__')

    def __exit__(self, kind, value, traceback):
        return self._magic['__exit__'](kind, value, traceback)


class _Patch:
    def __init__(self, target, attribute, new, create, made):
        self.target = target
        self.attribute = attribute
        self.new = new
        self.create = create
        self.made = made
        self.temporary = None
        self.held = DEFAULT

    def start(self):
        if hasattr(self.target, self.attribute):
            self.held = getattr(self.target, self.attribute)
        elif not self.create:
            raise AttributeError('the target has no attribute ' + repr(self.attribute))
        else:
            self.held = DEFAULT
        if self.new is DEFAULT:
            self.temporary = Mock(name=self.attribute, **self.made)
        else:
            self.temporary = self.new
        setattr(self.target, self.attribute, self.temporary)
        return self.temporary

    def stop(self):
        if self.held is DEFAULT:
            delattr(self.target, self.attribute)
        else:
            setattr(self.target, self.attribute, self.held)
        self.temporary = None

    def __enter__(self):
        return self.start()

    def __exit__(self, kind, value, traceback):
        self.stop()
        return False

    def __call__(self, function):
        if isinstance(function, type):
            raise 'NotImplementedError: standing over a whole class is not supported; put the patch on the test itself'
        patcher = self
        def patched(*args, **kwargs):
            fresh = patcher.start()
            try:
                if patcher.new is DEFAULT:
                    given = list(args)
                    given.append(fresh)
                    return function(*given, **kwargs)
                return function(*args, **kwargs)
            finally:
                patcher.stop()
        setattr(patched, '__name__', getattr(function, '__name__', 'patched'))
        return patched


_TAKEN = ('wraps', 'return_value', 'side_effect')

# A dotted name is split at its last dot: what comes before is a chain
# of modules and, where a plain attribute lookup does not reach that
# far, packages imported one more level at a time; what comes after is
# the attribute patch.object would have been given directly.
def _get_target(target):
    if not isinstance(target, str):
        raise TypeError('Need a valid target to patch. You supplied: ' + repr(target))
    modpath, sep, attribute = target.rpartition('.')
    if not sep:
        raise TypeError('Need a valid target to patch. You supplied: ' + repr(target))
    components = modpath.split('.')
    import_path = components[0]
    thing = __import__(import_path)
    for comp in components[1:]:
        import_path += '.' + comp
        if hasattr(thing, comp):
            thing = getattr(thing, comp)
        else:
            thing = __import__(import_path)
    return thing, attribute

class _Patcher:
    def __call__(self, target, new=DEFAULT, create=False, **extra):
        for word in extra:
            if word not in _TAKEN:
                raise 'NotImplementedError: patch does not take ' + word
        thing, attribute = _get_target(target)
        return _Patch(thing, attribute, new, create, extra)

    def object(self, target, attribute, new=DEFAULT, create=False, **extra):
        for word in extra:
            if word not in _TAKEN:
                raise 'NotImplementedError: patch.object does not take ' + word
        return _Patch(target, attribute, new, create, extra)

    def dict(self, target, values=None, clear=False, **extra):
        raise 'NotImplementedError: patch.dict is not supported'

    def multiple(self, target, **extra):
        raise 'NotImplementedError: patch.multiple is not supported'

patch = _Patcher()


class _OpenHandle(MagicMock):
    def __init__(self, read_data):
        MagicMock.__init__(self, name='handle')
        self._read_data = read_data
        self._position = 0
        self.read = Mock(name='read', side_effect=self._read)
        self.readline = Mock(name='readline', side_effect=self._readline)
        self.readlines = Mock(name='readlines', side_effect=self._readlines)
        self.write = Mock(name='write', return_value=None)
        self.close = Mock(name='close', return_value=None)
        self._magic['__enter__'] = Mock(name='__enter__', return_value=self)
        self._magic['__iter__'].side_effect = self._iterate

    def __enter__(self):
        return self._magic['__enter__']()

    def _read(self, size=-1):
        start = self._position
        end = len(self._read_data) if size is None or size < 0 else min(start + size, len(self._read_data))
        self._position = end
        return self._read_data[start:end]

    def _readline(self, size=-1):
        start = self._position
        newline = b'\n' if isinstance(self._read_data, bytes) else '\n'
        end = self._read_data.find(newline, start)
        end = len(self._read_data) if end < 0 else end + 1
        if size is not None and size >= 0:
            end = min(end, start + size)
        self._position = end
        return self._read_data[start:end]

    def _readlines(self, hint=-1):
        result = []
        total = 0
        while self._position < len(self._read_data):
            line = self._readline()
            result.append(line)
            total += len(line)
            if hint > 0 and total >= hint:
                break
        return result

    def _iterate(self):
        while self._position < len(self._read_data):
            yield self.readline()


def mock_open(mock=None, read_data=''):
    if not isinstance(read_data, (str, bytes)):
        raise TypeError('initial_value must be str or bytes, not ' + type(read_data).__name__)
    if mock is None:
        mock = MagicMock(name='open')
    handle = _OpenHandle(read_data)
    mock.return_value = handle
    def reset_data(*args, **kwargs):
        handle._position = 0
        return DEFAULT
    mock.side_effect = reset_data
    return mock


def __getattr__(name):
    raise 'NotImplementedError: unittest.mock.' + name + ' is not supported'
