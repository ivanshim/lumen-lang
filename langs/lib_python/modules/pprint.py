# Compact representations preserve sequence punctuation and quote nested text.
from heapq import _ordered

def _join(parts, separator):
    text = ''
    for part in parts:
        if len(text):
            text += separator
        text += part
    return text

def pformat(object, indent=1, width=80, depth=None, compact=False, sort_dicts=True, underscore_numbers=False):
    if indent < 0 or width == 0 or (depth is not None and depth <= 0):
        raise 'ValueError: invalid formatting bounds'
    if depth is not None or underscore_numbers:
        raise 'NotImplementedError: depth and grouped numbers are not supported'
    if isinstance(object, dict):
        keys = list(object)
        if sort_dicts:
            keys = _ordered(keys)
        parts = [repr(k) + ': ' + repr(object[k]) for k in keys]
        text = '{' + _join(parts, ', ') + '}'
        if len(text) > width:
            text = '{' + _join(parts, ',\n' + ' ' * indent) + '}'
        return text
    return repr(object)

def pprint(object, stream=None, indent=1, width=80, depth=None, compact=False, sort_dicts=True, underscore_numbers=False):
    text = pformat(object, indent, width, depth, compact, sort_dicts, underscore_numbers)
    if stream is None:
        print(text)
    else:
        stream.write(text + '\n')

class PrettyPrinter:
    def __init__(self, indent=1, width=80, depth=None, stream=None, compact=False, sort_dicts=True, underscore_numbers=False):
        self.indent = indent
        self.width = width
        self.depth = depth
        self.stream = stream
        self.compact = compact
        self.sort_dicts = sort_dicts
        self.underscore_numbers = underscore_numbers

    def pformat(self, object):
        return pformat(object, self.indent, self.width, self.depth, self.compact, self.sort_dicts, self.underscore_numbers)

    def pprint(self, object):
        text = self.pformat(object)
        if self.stream is None:
            print(text)
        else:
            self.stream.write(text + '\n')
