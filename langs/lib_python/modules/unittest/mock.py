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
        self.side_effect = side_effect
        self.called = False
        self.call_count = 0
        self.call_args = None
        self.call_args_list = []

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
                raise 'NotImplementedError: a side effect that is neither a fault nor a call is not supported'
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

class _Patcher:
    def __call__(self, target, new=DEFAULT, **extra):
        raise 'NotImplementedError: patch by dotted name needs a lookup this runtime has not; name the thing itself with patch.object'

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


def __getattr__(name):
    raise 'NotImplementedError: unittest.mock.' + name + ' is not supported'
