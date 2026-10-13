# The profiler behind cProfile, kept in lumen Python on top of
# sys.setprofile. CPython writes it in C (Modules/_lsprof.c) and counts
# every call and return the interpreter makes; this one is told of the
# calls and returns of routines written in Python and, with the builtins
# option, of builtin functions and methods (the c_call and c_return
# events), which become entries named as the C module names them. The
# sums are kept as the C module keeps them: a call counts once when it
# returns, a call inside itself adds to the recursive count, and the
# total time is taken only by the outermost of them.
import sys
from time import perf_counter

__all__ = ['Profiler', 'profiler_entry', 'profiler_subentry']


class _structseq(tuple):
    __slots__ = ()
    n_unnamed_fields = 0
    _names = ()

    def __new__(cls, sequence):
        values = tuple(sequence)
        if len(values) != len(cls._names):
            raise TypeError(cls.__module__ + '.' + cls.__name__ + '() takes a '
                            + str(len(cls._names)) + '-sequence ('
                            + str(len(values)) + '-sequence given)')
        return tuple.__new__(cls, values)

    def __repr__(self):
        inside = ', '.join(name + '=' + repr(self[at]) for at, name in enumerate(self._names))
        return self.__module__ + '.' + type(self).__name__ + '(' + inside + ')'

    def __reduce__(self):
        return (type(self), (tuple(self),))

    code = property(lambda self: self[0])
    callcount = property(lambda self: self[1])
    reccallcount = property(lambda self: self[2])
    totaltime = property(lambda self: self[3])
    inlinetime = property(lambda self: self[4])


class profiler_entry(_structseq):
    __slots__ = ()
    n_fields = 6
    n_sequence_fields = 6
    _names = ('code', 'callcount', 'reccallcount', 'totaltime', 'inlinetime', 'calls')
    __match_args__ = _names
    calls = property(lambda self: self[5])


class profiler_subentry(_structseq):
    __slots__ = ()
    n_fields = 5
    n_sequence_fields = 5
    _names = ('code', 'callcount', 'reccallcount', 'totaltime', 'inlinetime')
    __match_args__ = _names


profiler_entry.__module__ = '_lsprof'
profiler_subentry.__module__ = '_lsprof'


class _Tally:
    # What is known of one routine, or of one routine as called from
    # another: how often, how often inside itself, and the two times.
    def __init__(self, code):
        self.code = code
        self.calls = 0
        self.recursive = 0
        self.total = 0
        self.inline = 0
        self.open = 0
        self.subs = {}


class _Context:
    # A call under way. `frame` is the frame of a routine written in
    # Python, nothing for a builtin; `tally` is dropped by clear().
    def __init__(self, tally, key, before, began, frame):
        self.tally = tally
        self.key = key
        self.before = before
        self.began = began
        self.frame = frame
        self.beneath = 0


def _key_of(code):
    return (code.co_filename, code.co_firstlineno, code.co_name)


def _builtin_label(function):
    # The name the C module gives a builtin function or method.
    # Classes called are not builtin functions to the C module, nor is
    # a callable that has no name to give.
    if isinstance(function, type):
        return None
    name = getattr(function, '__name__', None)
    if not name:
        return None
    holder = getattr(function, '__self__', None)
    if holder is None or type(holder).__name__ == 'module':
        module = getattr(function, '__module__', None)
        if module is None:
            qualified = getattr(function, '__qualname__', '')
            if '.' in qualified:
                return "<method '" + name + "' of '" + qualified.rsplit('.', 1)[0] + "' objects>"
            return '<built-in method ' + name + '>'
        return '<built-in method ' + module + '.' + name + '>'
    return "<method '" + name + "' of '" + type(holder).__name__ + "' objects>"


_METHODS = {}


class Profiler:
    """Build a profiler object using the specified timer function.

    The default timer is a fast built-in one based on real time.
    For custom timer functions returning integers, 'timeunit' can
    be a float specifying a scale (that is, how long each integer unit
    is, in seconds).
    """

    def __init__(self, timer=None, timeunit=0.0, subcalls=True, builtins=True):
        if timer is not None and not callable(timer):
            raise TypeError("'" + type(timer).__name__ + "' object is not callable")
        if not isinstance(timeunit, (int, float)):
            raise TypeError("must be real number, not " + type(timeunit).__name__)
        self._timer = timer if timer is not None else perf_counter
        self._scale = float(timeunit) if timeunit else 1.0
        self._subcalls = bool(subcalls)
        self._builtins = bool(builtins)
        self._enabled = False
        self._tallies = {}
        self._top = None
        self._watch = self._follow
        self._muted = 0

    def _follow(self, frame, event, argument):
        if self._muted:
            return
        if event == 'call':
            code = frame.f_code
            if code.co_filename == _HERE:
                # The methods of this class stand for the C module's own
                # methods; anything else of this file is not a call.
                label = _METHODS.get(code.co_name)
                if label is not None and self._builtins:
                    self._enter(('builtin', label), label, frame)
                return
            self._enter(_key_of(code), code, frame)
        elif event == 'return':
            top = self._top
            if top is not None and top.frame is frame:
                self._leave()
        elif event == 'c_call':
            if self._builtins:
                label = _builtin_label(argument)
                if label is not None:
                    self._enter(('builtin', label), label, None)
        elif event == 'c_return' or event == 'c_exception':
            top = self._top
            if top is not None and top.frame is None and top.key == ('builtin', _builtin_label(argument)):
                self._leave()

    def _enter(self, key, code, frame):
        tally = self._tallies.get(key)
        if tally is None:
            tally = self._tallies[key] = _Tally(code)
        tally.open += 1
        before = self._top
        if self._subcalls and before is not None and before.tally is not None:
            sub = before.tally.subs.get(key)
            if sub is None:
                sub = before.tally.subs[key] = _Tally(tally.code)
            sub.open += 1
        self._top = _Context(tally, key, before, self._timer(), frame)

    def _leave(self):
        context = self._top
        spent = self._timer() - context.began
        inline = spent - context.beneath
        before = context.before
        if before is not None:
            before.beneath += spent
        self._top = before
        tally = context.tally
        if tally is None:
            return
        tally.open -= 1
        if tally.open == 0:
            tally.total += spent
        else:
            tally.recursive += 1
        tally.inline += inline
        tally.calls += 1
        if self._subcalls and before is not None and before.tally is not None:
            sub = before.tally.subs.get(context.key)
            if sub is not None:
                sub.open -= 1
                if sub.open == 0:
                    sub.total += spent
                else:
                    sub.recursive += 1
                sub.inline += inline
                sub.calls += 1

    def _flush(self):
        while self._top is not None:
            self._leave()

    def enable(self, subcalls=True, builtins=True):
        """Start collecting profiling information.

        subcalls
          If True, also records for each function
          statistics separated according to its current caller.
        builtins
          If True, records the time spent in
          built-in functions separately from their caller.
        """
        self._muted += 1
        try:
            self._subcalls = bool(subcalls)
            self._builtins = bool(builtins)
            self._enabled = True
            sys.setprofile(self._watch)
        finally:
            self._muted -= 1

    def disable(self):
        """Stop collecting profiling information."""
        self._muted += 1
        try:
            if self._enabled:
                if sys.getprofile() == self._watch:
                    sys.setprofile(None)
                self._enabled = False
            self._flush()
        finally:
            self._muted -= 1

    def clear(self):
        """Clear all profiling information collected so far."""
        # Calls still under way are left to finish without being counted.
        context = self._top
        while context is not None:
            context.tally = None
            context = context.before
        self._tallies = {}

    def getstats(self):
        """list of profiler_entry objects.

        getstats() -> list of profiler_entry objects

        Return all information collected by the profiler.
        Each profiler_entry is a tuple-like object with the
        following attributes:

            code          code object
            callcount     how many times this was called
            reccallcount  how many times called recursively
            totaltime     total time in this entry
            inlinetime    inline time in this entry (not in subcalls)
            calls         details of the calls

        The calls attribute is either None or a list of
        profiler_subentry objects:

            code          called code object
            callcount     how many times this is called
            reccallcount  how many times this is called recursively
            totaltime     total time spent in this call
            inlinetime    inline time (not in further subcalls)
        """
        self._muted += 1
        try:
            scale = self._scale
            found = []
            for tally in list(self._tallies.values()):
                if tally.calls == 0:
                    continue
                subs = None
                if tally.subs:
                    subs = [profiler_subentry((sub.code, sub.calls, sub.recursive,
                                               sub.total * scale, sub.inline * scale))
                            for sub in tally.subs.values() if sub.calls]
                found.append(profiler_entry((tally.code, tally.calls, tally.recursive,
                                             tally.total * scale, tally.inline * scale, subs)))
        finally:
            self._muted -= 1
        return found


_HERE = Profiler.enable.__code__.co_filename
_METHODS.update({
    'enable': "<method 'enable' of '_lsprof.Profiler' objects>",
    'disable': "<method 'disable' of '_lsprof.Profiler' objects>",
    'clear': "<method 'clear' of '_lsprof.Profiler' objects>",
    'getstats': "<method 'getstats' of '_lsprof.Profiler' objects>",
})
