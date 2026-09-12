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
