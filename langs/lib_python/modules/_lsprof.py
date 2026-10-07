# The profiler behind cProfile, kept in lumen Python on top of
# sys.setprofile. CPython writes it in C (Modules/_lsprof.c) and counts
# every call and return the interpreter makes, builtin functions
# included; this one is told of the calls and returns of the routines
# written in Python and nothing else, so the built-in functions a
# program calls are not entries of their own (their time stays with the
# caller). The sums are kept as the C module keeps them: a call counts
# once when it returns, a call inside itself adds to the recursive
# count, and the total time is taken only by the outermost of them.
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
    def __init__(self, tally, before, began):
        self.tally = tally
        self.before = before
        self.began = began
        self.beneath = 0


def _key_of(code):
    return (code.co_filename, code.co_firstlineno, code.co_name)


_HERE = None


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

    def _follow(self, frame, event, argument):
        if event == 'call':
            code = frame.f_code
            if code.co_filename == _HERE:
                return
            self._enter(code)
        elif event == 'return':
            if self._top is not None and frame.f_code.co_filename != _HERE:
                self._leave()

    def _enter(self, code):
        key = _key_of(code)
        tally = self._tallies.get(key)
        if tally is None:
            tally = self._tallies[key] = _Tally(code)
        tally.open += 1
        before = self._top
        if self._subcalls and before is not None:
            sub = before.tally.subs.get(key)
            if sub is None:
                sub = before.tally.subs[key] = _Tally(code)
            sub.open += 1
        self._top = _Context(tally, before, self._timer())

    def _leave(self):
        context = self._top
        spent = self._timer() - context.began
        inline = spent - context.beneath
        before = context.before
        if before is not None:
            before.beneath += spent
        self._top = before
        tally = context.tally
        tally.open -= 1
        if tally.open == 0:
            tally.total += spent
        else:
            tally.recursive += 1
        tally.inline += inline
        tally.calls += 1
        if self._subcalls and before is not None:
            sub = before.tally.subs[_key_of(tally.code)]
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
        self._subcalls = bool(subcalls)
        self._builtins = bool(builtins)
        self._enabled = True
        sys.setprofile(self._watch)

    def disable(self):
        """Stop collecting profiling information."""
        if self._enabled:
            if sys.getprofile() == self._watch:
                sys.setprofile(None)
            self._enabled = False
        self._flush()

    def clear(self):
        """Clear all profiling information collected so far."""
        self._flush()
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
        self._flush()
        scale = self._scale
        found = []
        for tally in self._tallies.values():
            if tally.calls == 0:
                continue
            subs = None
            if tally.subs:
                subs = [profiler_subentry((sub.code, sub.calls, sub.recursive,
                                           sub.total * scale, sub.inline * scale))
                        for sub in tally.subs.values() if sub.calls]
            found.append(profiler_entry((tally.code, tally.calls, tally.recursive,
                                         tally.total * scale, tally.inline * scale, subs)))
        return found


_HERE = Profiler.enable.__code__.co_filename
