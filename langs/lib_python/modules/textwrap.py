# Whitespace is measured in characters; common leading space is removed.
def dedent(text):
    lines = _lines(text)
    margin = None
    for line in lines:
        if _trim(line, ' \t') != '':
            prefix = ''
            for letter in list(line):
                if letter != ' ' and letter != '\t':
                    break
                prefix += letter
            if margin is None:
                margin = prefix
            else:
                while prefix[:len(margin)] != margin:
                    margin = margin[:-1]
    result = []
    for line in lines:
        if _trim(line, ' \t') == '':
            result.append('')
        else:
            result.append(line[len(margin):])
    return _join(result)

def indent(text, prefix, predicate=None):
    result = ''
    lines = _lines(text)
    for i in range(len(lines)):
        line = lines[i]
        if i < len(lines) - 1:
            line += '\n'
        if predicate is None:
            wanted = _trim(line, ' \t\n\r\v\f') != ''
        else:
            wanted = predicate(line)
        result += prefix + line if wanted else line
    return result

# Stub: the small wrapper carries the usual width and indentation only.
# Hyphen breaking and the wider TextWrapper options are not provided.
def wrap(text, width=70, initial_indent='', subsequent_indent='', **options):
    if len(options):
        raise 'NotImplementedError: these wrapping options are not supported'
    if width <= 0:
        raise 'ValueError: invalid width'
    lines = []
    line = initial_indent
    prefix = initial_indent
    for word in _words(text):
        if len(line) > len(prefix):
            if len(line) + 1 + len(word) > width:
                lines.append(line)
                prefix = subsequent_indent
                line = prefix
            else:
                line += ' '
        while len(line) + len(word) > width:
            room = width - len(line)
            if room <= 0:
                raise 'ValueError: indentation exceeds width'
            lines.append(line + word[:room])
            word = word[room:]
            prefix = subsequent_indent
            line = prefix
        line += word
    if len(line) > len(prefix):
        lines.append(line)
    return lines

def fill(text, width=70, **options):
    return _join(wrap(text, width, **options))


def _lines(text):
    result = []
    part = ''
    for letter in list(text):
        if letter == '\n':
            result.append(part)
            part = ''
        else:
            part += letter
    result.append(part)
    return result

def _join(parts):
    result = ''
    for i in range(len(parts)):
        if i:
            result += '\n'
        result += parts[i]
    return result

def _trim(text, spaces):
    start = 0
    end = len(text)
    while start < end and text[start] in spaces:
        start += 1
    while end > start and text[end - 1] in spaces:
        end -= 1
    return text[start:end]

def _words(text):
    result = []
    part = ''
    for letter in list(text + ' '):
        if letter in ' \t\n\r\v\f':
            if part != '':
                result.append(part)
                part = ''
        else:
            part += letter
    return result
