# The fixed ASCII alphabets need no system tables.
ascii_lowercase = 'abcdefghijklmnopqrstuvwxyz'
ascii_uppercase = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'
ascii_letters = ascii_lowercase + ascii_uppercase
digits = '0123456789'
hexdigits = digits + 'abcdefABCDEF'
octdigits = '01234567'
punctuation = '!"#$%&\'()*+,-./:;<=>?@[\\]^_`{|}~'
whitespace = ' \t\n\r\v\f'
printable = digits + ascii_letters + punctuation + whitespace

# Stub: subclasses with different delimiters or patterns are not supported.
class Template:
    def __init__(self, template):
        self.template = template

    def substitute(self, mapping=None, **keywords):
        return self._replace(mapping, keywords, False)

    def safe_substitute(self, mapping=None, **keywords):
        return self._replace(mapping, keywords, True)

    def _replace(self, mapping, keywords, safe):
        if mapping is None:
            mapping = {}
        text = self.template
        result = ''
        i = 0
        while i < len(text):
            if text[i] != '$':
                result += text[i]
                i += 1
                continue
            begin = i
            i += 1
            if i < len(text) and text[i] == '$':
                result += '$'
                i += 1
                continue
            brace = i < len(text) and text[i] == '{'
            if brace:
                i += 1
            name = ''
            if i < len(text) and text[i] in ascii_letters + '_':
                while i < len(text) and text[i] in ascii_letters + digits + '_':
                    name += text[i]
                    i += 1
            valid = name != ''
            if brace:
                if i < len(text) and text[i] == '}':
                    i += 1
                else:
                    valid = False
            if not valid:
                if not safe:
                    raise 'ValueError: invalid placeholder in string'
                result += text[begin:i]
            elif name in keywords:
                result += str(keywords[name])
            elif name in mapping:
                result += str(mapping[name])
            elif safe:
                result += text[begin:i]
            else:
                raise 'KeyError: ' + name
        return result

def capwords(s, sep=None):
    words = s.split(sep)
    result = []
    for word in words:
        result.append(word[:1].upper() + word[1:].lower())
    return (' ' if sep is None else sep).join(result)

class Formatter:
    def format(self, format_string, *args, **kwargs):
        return self.vformat(format_string, args, kwargs)

    def vformat(self, format_string, args, kwargs):
        # The text formatter itself owns conversion and format clauses.
        return format_string.format(*args, **kwargs)
