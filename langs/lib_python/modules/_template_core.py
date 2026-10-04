# Native template string storage, instantiated by the template prefix labels.
_load = __storage_get
_store = __storage_set

class Interpolation:
    __slots__ = ()
    __match_args__ = ('value', 'expression', 'conversion', 'format_spec')
    def __init__(self, value, expression, conversion=None, format_spec=''):
        if not isinstance(expression, str) or not isinstance(format_spec, str):
            raise TypeError('expression and format_spec must be strings')
        if conversion not in (None, 'a', 'r', 's'):
            raise ValueError('conversion must be None, a, r, or s')
        _store(self, '\0value', value)
        _store(self, '\0expression', expression)
        _store(self, '\0conversion', conversion)
        _store(self, '\0format_spec', format_spec)
        __seal_class(self)

    @property
    def value(self):
        return _load(self, '\0value')

    @property
    def expression(self):
        return _load(self, '\0expression')

    @property
    def conversion(self):
        return _load(self, '\0conversion')

    @property
    def format_spec(self):
        return _load(self, '\0format_spec')

    def __repr__(self):
        return 'Interpolation(' + repr(self.value) + ', ' + repr(self.expression) + ', ' + repr(self.conversion) + ', ' + repr(self.format_spec) + ')'


class Template:
    __slots__ = ()
    def __init__(self, *args):
        strings = ['']
        interpolations = []
        for arg in args:
            if isinstance(arg, str):
                strings[-1] += arg
            elif isinstance(arg, Interpolation):
                interpolations.append(arg)
                strings.append('')
            else:
                raise TypeError('Template arguments must be strings or interpolations')
        _store(self, '\0strings', tuple(strings))
        _store(self, '\0interpolations', tuple(interpolations))
        __seal_class(self)

    @property
    def strings(self):
        return _load(self, '\0strings')

    @property
    def interpolations(self):
        return _load(self, '\0interpolations')

    @property
    def values(self):
        return tuple(item.value for item in self.interpolations)

    def __iter__(self):
        for index, string in enumerate(self.strings):
            if string:
                yield string
            if index < len(self.interpolations):
                yield self.interpolations[index]

    def __add__(self, other):
        if not isinstance(other, Template):
            return NotImplemented
        return Template(*list(self), *list(other))

    def __repr__(self):
        return 'Template(strings=' + repr(self.strings) + ', interpolations=' + repr(self.interpolations) + ')'


def convert(value, conversion):
    if conversion == 'a':
        return ascii(value)
    if conversion == 'r':
        return repr(value)
    if conversion == 's':
        return str(value)
    if conversion is None:
        return value
    raise ValueError('invalid conversion')

Template.__module__ = 'string.templatelib'
Interpolation.__module__ = 'string.templatelib'
__seal_class(Template)
__seal_class(Interpolation)
