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
    words = []
    word = ''
    i = 0
    if sep == '':
        raise 'ValueError: empty separator'
    while i < len(s):
        divides = s[i] in whitespace if sep is None else s[i:i + len(sep)] == sep
        if divides:
            if word != '' or sep is not None:
                words.append(word)
            word = ''
            i += 1 if sep is None else len(sep)
        else:
            word += s[i]
            i += 1
    if word != '' or sep is not None:
        words.append(word)
    result = ''
    for index in range(len(words)):
        if index:
            result += ' ' if sep is None else sep
        word = words[index]
        for j in range(len(word)):
            ch = word[j]
            if ord(ch) > 127:
                raise 'NotImplementedError: capwords supports ASCII case conversion'
            if j == 0 and ch in ascii_lowercase:
                ch = chr(ord(ch) - 32)
            elif j != 0 and ch in ascii_uppercase:
                ch = chr(ord(ch) + 32)
            result += ch
    return result

class Formatter:
    def format(self, format_string, *args, **kwargs):
        return self.vformat(format_string, args, kwargs)

    def vformat(self, format_string, args, kwargs):
        result = ''
        i = 0
        automatic = 0
        numbering = ''
        while i < len(format_string):
            ch = format_string[i]
            if ch not in '{}':
                result += ch
                i += 1
                continue
            if i + 1 < len(format_string) and format_string[i + 1] == ch:
                result += ch
                i += 2
                continue
            if ch == '}':
                raise "ValueError: single '}' encountered in format string"
            i += 1
            name = ''
            while i < len(format_string) and format_string[i] != '}':
                name += format_string[i]
                i += 1
            if i == len(format_string):
                raise "ValueError: expected '}' before end of string"
            i += 1
            if name == '':
                if numbering == 'manual':
                    raise 'ValueError: cannot switch from manual to automatic numbering'
                numbering = 'automatic'
                if automatic >= len(args):
                    raise 'IndexError: replacement index out of range'
                result += str(args[automatic])
                automatic += 1
            elif name in kwargs:
                result += str(kwargs[name])
            else:
                if numbering == 'automatic':
                    raise 'ValueError: cannot switch from automatic to manual numbering'
                numbering = 'manual'
                number = 0
                for letter in name:
                    if letter not in digits:
                        raise 'NotImplementedError: Formatter supports only plain replacement fields'
                    number = number * 10 + ord(letter) - 48
                if number >= len(args):
                    raise 'IndexError: replacement index out of range'
                result += str(args[number])
        return result
