# Template and interpolation values for Python's template string syntax.
class Interpolation:
    def __init__(self, value, expression, conversion=None, format_spec=''):
        if not isinstance(expression, str) or not isinstance(format_spec, str):
            raise TypeError('expression and format_spec must be strings')
        if conversion not in (None, 'a', 'r', 's'):
            raise ValueError('conversion must be None, a, r, or s')
        self._value = value
        self._expression = expression
        self._conversion = conversion
        self._format_spec = format_spec

    @property
    def value(self):
        return self._value

    @property
    def expression(self):
        return self._expression

    @property
    def conversion(self):
        return self._conversion

    @property
    def format_spec(self):
        return self._format_spec

    def __repr__(self):
        return 'Interpolation(' + repr(self.value) + ', ' + repr(self.expression) + ', ' + repr(self.conversion) + ', ' + repr(self.format_spec) + ')'


class Template:
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
        self._strings = tuple(strings)
        self._interpolations = tuple(interpolations)

    @property
    def strings(self):
        return self._strings

    @property
    def interpolations(self):
        return self._interpolations

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
