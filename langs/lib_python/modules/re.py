# A backtracking reader over text. Backreferences, inline flags, bytes and
# locale rules are not carried by this small engine. Unsupported escapes
# and group forms raise a complaint when compiled.
I = 2
IGNORECASE = I
M = 8
MULTILINE = M
S = 16
DOTALL = S
X = 64
VERBOSE = X

_DIGITS = '0123456789'
_WORD_LETTERS = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_'

class _Reader:
    def __init__(self, pattern):
        self.pattern = pattern
        self.i = 0
        self.groups = 0
        self.names = {}

    def choice(self):
        arms = [self.sequence()]
        while self.i < len(self.pattern) and self.pattern[self.i] == '|':
            self.i += 1
            arms.append(self.sequence())
        return ['or', arms]

    def sequence(self):
        nodes = []
        while self.i < len(self.pattern) and self.pattern[self.i] not in ')|':
            before = self.i
            node = self.atom()
            if self.i <= before:
                raise 'RuntimeError: regular expression reader made no progress'
            if self.i < len(self.pattern) and self.pattern[self.i] in '*+?{':
                applied = self._try_quantifier(node)
                if applied is not None:
                    node = applied
            nodes.append(node)
        return ['seq', nodes]

    def _try_quantifier(self, node):
        mark = self.pattern[self.i]
        if mark == '{':
            spec = self._repeat_spec(self.i)
            if spec is None:
                return None
            low, high, newi = spec
            if high >= 0 and high < low:
                raise 'ValueError: min repeat greater than max repeat'
            self.i = newi
        else:
            self.i += 1
            low = 1 if mark == '+' else 0
            high = 1 if mark == '?' else -1
        greedy = True
        if self.i < len(self.pattern) and self.pattern[self.i] == '?':
            greedy = False
            self.i += 1
        return ['repeat', node, low, high, greedy]

    def _repeat_spec(self, i):
        # `i` names the position of the opening `{`. Returns (low, high,
        # index just past the closing `}`), or None when what follows does
        # not read as a repetition -- CPython then takes the brace as a
        # literal character rather than as a complaint.
        n = len(self.pattern)
        j = i + 1
        low_start = j
        while j < n and self.pattern[j] in _DIGITS:
            j += 1
        low_text = self.pattern[low_start:j]
        has_comma = j < n and self.pattern[j] == ','
        if has_comma:
            j += 1
            high_start = j
            while j < n and self.pattern[j] in _DIGITS:
                j += 1
            high_text = self.pattern[high_start:j]
        else:
            high_text = low_text
        if j >= n or self.pattern[j] != '}':
            return None
        if not has_comma and low_text == '':
            return None
        low = int(low_text) if low_text != '' else 0
        if has_comma:
            high = int(high_text) if high_text != '' else -1
        else:
            high = low
        return (low, high, j + 1)

    def escaped(self, inside=False):
        if self.i >= len(self.pattern):
            raise 'ValueError: trailing backslash'
        mark = self.pattern[self.i]
        self.i += 1
        if mark in 'dDwWsS':
            return ['kind', mark]
        if mark == 'b':
            return ['lit', '\b'] if inside else ['boundary']
        if mark == 'A':
            return ['lit', 'A'] if inside else ['start_abs']
        if mark == 'Z':
            return ['lit', 'Z'] if inside else ['end_abs']
        if mark == 'n':
            return ['lit', '\n']
        if mark == 't':
            return ['lit', '\t']
        if mark == 'r':
            return ['lit', '\r']
        if mark == 'f':
            return ['lit', '\f']
        if mark == 'v':
            return ['lit', '\v']
        if not inside and mark in '123456789':
            return ['backref', int(mark)]
        if mark in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789':
            raise 'NotImplementedError: this regular expression escape is not supported'
        return ['lit', mark]

    def atom(self):
        mark = self.pattern[self.i]
        self.i += 1
        if mark == '(':
            return self._group()
        if mark == '[':
            return self._class()
        if mark == '\\':
            return self.escaped()
        if mark == '.':
            return ['dot']
        if mark == '^':
            return ['start']
        if mark == '$':
            return ['end']
        if mark in '*+?':
            raise 'ValueError: nothing to repeat'
        if mark == '{':
            spec = self._repeat_spec(self.i - 1)
            if spec is not None:
                low, high, _unused = spec
                if high >= 0 and high < low:
                    raise 'ValueError: min repeat greater than max repeat'
                raise 'ValueError: nothing to repeat'
            return ['lit', '{']
        if mark == '}':
            return ['lit', '}']
        return ['lit', mark]

    def _group(self):
        if self.i < len(self.pattern) and self.pattern[self.i] == '?':
            self.i += 1
            if self.i >= len(self.pattern):
                raise 'ValueError: unexpected end of pattern'
            marker = self.pattern[self.i]
            if marker == ':':
                self.i += 1
                node = self.choice()
                self._close_group()
                return node
            if marker == '=' or marker == '!':
                self.i += 1
                node = self.choice()
                self._close_group()
                return ['lookahead', marker == '=', node]
            if marker == '<' and self.i + 1 < len(self.pattern) and self.pattern[self.i + 1] in '=!':
                self.i += 1
                positive = self.pattern[self.i] == '='
                self.i += 1
                node = self.choice()
                self._close_group()
                return ['lookbehind', positive, node]
            if marker == 'P':
                self.i += 1
                if self.i < len(self.pattern) and self.pattern[self.i] == '<':
                    self.i += 1
                    start = self.i
                    while self.i < len(self.pattern) and self.pattern[self.i] != '>':
                        self.i += 1
                    if self.i == len(self.pattern):
                        raise 'ValueError: missing >, unterminated name'
                    name = self.pattern[start:self.i]
                    self.i += 1
                    self.groups += 1
                    number = self.groups
                    self.names[name] = number
                    node = self.choice()
                    self._close_group()
                    return ['group', number, node]
                if self.i < len(self.pattern) and self.pattern[self.i] == '=':
                    self.i += 1
                    start = self.i
                    while self.i < len(self.pattern) and self.pattern[self.i] != ')':
                        self.i += 1
                    if self.i == len(self.pattern):
                        raise 'ValueError: missing ), unterminated name'
                    name = self.pattern[start:self.i]
                    self.i += 1
                    if name not in self.names:
                        raise 'IndexError: unknown group name ' + repr(name)
                    return ['backref', self.names[name]]
                raise 'NotImplementedError: this regular expression group form is not supported'
            raise 'NotImplementedError: this regular expression group form is not supported'
        self.groups += 1
        number = self.groups
        node = self.choice()
        self._close_group()
        return ['group', number, node]

    def _close_group(self):
        if self.i == len(self.pattern) or self.pattern[self.i] != ')':
            raise 'ValueError: unterminated group'
        self.i += 1

    def _class(self):
        reverse = False
        if self.i < len(self.pattern) and self.pattern[self.i] == '^':
            self.i += 1
            reverse = True
        entries = []
        while self.i < len(self.pattern) and (self.pattern[self.i] != ']' or len(entries) == 0):
            letter = self.pattern[self.i]
            self.i += 1
            entry = self.escaped(True) if letter == '\\' else ['lit', letter]
            if self.i + 1 < len(self.pattern) and self.pattern[self.i] == '-' and self.pattern[self.i + 1] != ']':
                self.i += 1
                last = self.pattern[self.i]
                self.i += 1
                end = self.escaped(True) if last == '\\' else ['lit', last]
                if entry[0] != 'lit' or end[0] != 'lit' or ord(entry[1]) > ord(end[1]):
                    raise 'ValueError: bad character range'
                entry = ['range', entry[1], end[1]]
            entries.append(entry)
        if self.i == len(self.pattern):
            raise 'ValueError: unterminated character class'
        self.i += 1
        return ['class', reverse, entries]

def _strip_verbose(pattern):
    # VERBOSE drops whitespace and `#` comments that fall outside a
    # character class and outside a backslash escape, before the pattern
    # ever reaches the reader.
    result = ''
    i = 0
    n = len(pattern)
    in_class = False
    while i < n:
        c = pattern[i]
        if c == '\\' and i + 1 < n:
            result += pattern[i:i + 2]
            i += 2
            continue
        if not in_class and c == '[':
            in_class = True
            result += c
            i += 1
            continue
        if in_class and c == ']':
            in_class = False
            result += c
            i += 1
            continue
        if not in_class and c == '#':
            while i < n and pattern[i] != '\n':
                i += 1
            continue
        if not in_class and c in ' \t\n\r\v\f':
            i += 1
            continue
        result += c
        i += 1
    return result

def _word(letter):
    return letter != '' and letter in _WORD_LETTERS

def _accept(node, letter, ignorecase):
    kind = node[0]
    if kind == 'lit':
        if ignorecase:
            return letter.lower() == node[1].lower()
        return letter == node[1]
    if kind == 'range':
        if ignorecase:
            for candidate in [letter, letter.lower(), letter.upper()]:
                if ord(node[1]) <= ord(candidate) and ord(candidate) <= ord(node[2]):
                    return True
            return False
        return ord(node[1]) <= ord(letter) and ord(letter) <= ord(node[2])
    if kind == 'kind':
        mark = node[1]
        if mark in 'dD':
            answer = letter in _DIGITS
        elif mark in 'wW':
            answer = _word(letter)
        else:
            answer = letter in ' \t\n\r\v\f'
        return not answer if mark in 'DWS' else answer
    return False

def _walk(node, text, place, captures, opts):
    kind = node[0]
    if kind == 'seq':
        states = [[place, captures]]
        for part in node[1]:
            if len(states) == 0:
                break
            next_states = []
            for state in states:
                next_states = [*next_states, *_walk(part, text, state[0], state[1], opts)]
            states = next_states
        return states
    if kind == 'or':
        states = []
        for arm in node[1]:
            states = [*states, *_walk(arm, text, place, captures, opts)]
        return states
    if kind == 'group':
        states = []
        for state in _walk(node[2], text, place, captures, opts):
            found = __copy_value(state[1], False)
            found[node[1]] = [place, state[0]]
            states.append([state[0], found])
        return states
    if kind == 'repeat':
        return _repeat(node, text, place, captures, 0, opts)
    if kind == 'lookahead':
        states = _walk(node[2], text, place, captures, opts)
        if node[1]:
            return [[place, states[0][1]]] if len(states) else []
        return [] if len(states) else [[place, captures]]
    if kind == 'lookbehind':
        found = None
        s = place
        while s >= 0:
            for state in _walk(node[2], text, s, captures, opts):
                if state[0] == place:
                    found = state[1]
            if found is not None:
                break
            s -= 1
        if node[1]:
            return [[place, found]] if found is not None else []
        return [] if found is not None else [[place, captures]]
    if kind == 'backref':
        number = node[1]
        if number not in captures:
            return []
        span = captures[number]
        piece = text[span[0]:span[1]]
        width = len(piece)
        candidate = text[place:place + width]
        matched = candidate.lower() == piece.lower() if opts[0] else candidate == piece
        return [[place + width, captures]] if matched and place + width <= len(text) else []
    if kind == 'start':
        if place == 0:
            return [[place, captures]]
        if opts[1] and text[place - 1] == '\n':
            return [[place, captures]]
        return []
    if kind == 'start_abs':
        return [[place, captures]] if place == 0 else []
    if kind == 'end':
        if place == len(text):
            return [[place, captures]]
        if text[place] == '\n' and (opts[1] or place == len(text) - 1):
            return [[place, captures]]
        return []
    if kind == 'end_abs':
        return [[place, captures]] if place == len(text) else []
    if kind == 'boundary':
        before = _word(text[place - 1]) if place > 0 else False
        after = _word(text[place]) if place < len(text) else False
        return [[place, captures]] if before != after else []
    if place >= len(text):
        return []
    letter = text[place]
    if kind == 'dot':
        answer = opts[2] or letter != '\n'
    elif kind == 'class':
        answer = False
        for entry in node[2]:
            if _accept(entry, letter, opts[0]):
                answer = True
        if node[1]:
            answer = not answer
    else:
        answer = _accept(node, letter, opts[0])
    return [[place + 1, captures]] if answer else []

def _repeat(node, text, place, captures, count, opts):
    states = []
    if count >= node[2] and not node[4]:
        states.append([place, captures])
    if node[3] < 0 or count < node[3]:
        for state in _walk(node[1], text, place, captures, opts):
            if state[0] != place:
                states = [*states, *_repeat(node, text, state[0], state[1], count + 1, opts)]
            elif count + 1 >= node[2]:
                states.append(state)
            elif count < node[2]:
                states = [*states, *_repeat(node, text, place, state[1], count + 1, opts)]
    if count >= node[2] and node[4]:
        states.append([place, captures])
    return states

class Match:
    def __init__(self, pattern, text, start, state):
        self.re = pattern
        self.string = text
        captures = __copy_value(state[1], False)
        captures[0] = [start, state[0]]
        self.captures = captures

    def _index(self, group):
        if type(group) == type(''):
            if group not in self.re.groupindex:
                raise 'IndexError: no such group'
            return self.re.groupindex[group]
        return group

    def group(self, *numbers):
        if len(numbers) == 0:
            numbers = [0]
        values = []
        for number in numbers:
            number = self._index(number)
            if number < 0 or number > self.re.groups:
                raise 'IndexError: no such group'
            if number in self.captures:
                span = self.captures[number]
                values.append(self.string[span[0]:span[1]])
            else:
                values.append(None)
        return values[0] if len(values) == 1 else values

    def start(self, group=0):
        group = self._index(group)
        return self.captures[group][0] if group in self.captures else -1

    def end(self, group=0):
        group = self._index(group)
        return self.captures[group][1] if group in self.captures else -1

    def span(self, group=0):
        return (self.start(group), self.end(group))

    def groups(self, default=None):
        values = []
        for i in range(1, self.re.groups + 1):
            value = self.group(i)
            values.append(default if value is None else value)
        return tuple(values)

    def groupdict(self, default=None):
        result = {}
        for name in self.re.groupindex:
            value = self.group(self.re.groupindex[name])
            result[name] = default if value is None else value
        return result

class _FindIter:
    def __init__(self, pattern, string, pos, endpos):
        self.pattern = pattern
        self.string = string if endpos is None else string[:endpos]
        self.pos = pos
        self.empty_at = -1
        self.done = False

    def __iter__(self):
        return self

    def __next__(self):
        if self.done:
            raise StopIteration
        found = self.pattern._next(self.string, self.pos, self.empty_at)
        if found is None:
            self.done = True
            raise StopIteration
        self.pos = found.end()
        self.empty_at = self.pos if found.start() == self.pos else -1
        return found

def _expand(repl, found):
    result = ''
    i = 0
    n = len(repl)
    while i < n:
        c = repl[i]
        if c == '\\' and i + 1 < n:
            nxt = repl[i + 1]
            if nxt in _DIGITS:
                result += found.group(int(nxt))
                i += 2
                continue
            if nxt == 'g' and i + 2 < n and repl[i + 2] == '<':
                start = i + 3
                j = start
                while j < n and repl[j] != '>':
                    j += 1
                if j == n:
                    raise 'error: missing >, unterminated name'
                name = repl[start:j]
                group = int(name) if name.isdigit() else name
                result += found.group(group)
                i = j + 1
                continue
            if nxt == '\\':
                result += '\\'
                i += 2
                continue
            result += nxt
            i += 2
            continue
        result += c
        i += 1
    return result

class Pattern:
    def __init__(self, pattern, flags=0):
        self.flags = flags
        self.ignorecase = (flags & IGNORECASE) != 0
        self.multiline = (flags & MULTILINE) != 0
        self.dotall = (flags & DOTALL) != 0
        self.verbose = (flags & VERBOSE) != 0
        text = _strip_verbose(pattern) if self.verbose else pattern
        reader = _Reader(text)
        self.tree = reader.choice()
        if reader.i != len(text):
            raise 'ValueError: unbalanced parenthesis'
        self.groups = reader.groups
        self.groupindex = reader.names
        self.pattern = pattern
        self._opts = (self.ignorecase, self.multiline, self.dotall)
        # Whether the text a match runs against must be checked for
        # non-ASCII letters, computed once here rather than rescanning
        # the pattern's own text on every position a search tries.
        shorthand = False
        for mark in ['\\w', '\\W', '\\d', '\\D', '\\s', '\\S', '\\b']:
            if mark in pattern:
                shorthand = True
        self._shorthand = shorthand

    def _check_text(self, string):
        if not self._shorthand:
            return
        for letter in list(string):
            if ord(letter) > 127:
                raise 'NotImplementedError: Unicode shorthand classes are not supported'

    def match(self, string, pos=0, endpos=None):
        self._check_text(string)
        return self._match_at(string, pos, endpos)

    def _match_at(self, string, pos=0, endpos=None):
        # The part of `match` a caller who already checked the text
        # (`search`, `_next`) may call directly, at every position it
        # tries, without paying for that check again each time.
        if endpos is not None:
            string = string[:endpos]
        states = _walk(self.tree, string, pos, {}, self._opts)
        return Match(self, string, pos, states[0]) if len(states) else None

    def fullmatch(self, string, pos=0, endpos=None):
        self._check_text(string)
        if endpos is not None:
            string = string[:endpos]
        for state in _walk(self.tree, string, pos, {}, self._opts):
            if state[0] == len(string):
                return Match(self, string, pos, state)
        return None

    def search(self, string, pos=0, endpos=None):
        self._check_text(string)
        if endpos is not None:
            string = string[:endpos]
        while pos <= len(string):
            found = self._match_at(string, pos)
            if found is not None:
                return found
            pos += 1
        return None

    def _next(self, string, pos, empty_at):
        self._check_text(string)
        while pos <= len(string):
            for state in _walk(self.tree, string, pos, {}, self._opts):
                if pos != empty_at or state[0] != pos:
                    return Match(self, string, pos, state)
            pos += 1
        return None

    def finditer(self, string, pos=0, endpos=None):
        return _FindIter(self, string, pos, endpos)

    def findall(self, string, pos=0, endpos=None):
        if endpos is not None:
            string = string[:endpos]
        result = []
        empty_at = -1
        while pos <= len(string):
            found = self._next(string, pos, empty_at)
            if found is None:
                break
            if self.groups == 0:
                result.append(found.group())
            elif self.groups == 1:
                value = found.group(1)
                result.append('' if value is None else value)
            else:
                result.append(found.groups(''))
            pos = found.end()
            empty_at = pos if found.start() == pos else -1
        return result

    def sub(self, repl, string, count=0):
        is_func = callable(repl)
        result = ''
        previous = 0
        pos = 0
        used = 0
        empty_at = -1
        while pos <= len(string) and (count == 0 or used < count):
            found = self._next(string, pos, empty_at)
            if found is None:
                break
            result += string[previous:found.start()]
            result += repl(found) if is_func else _expand(repl, found)
            previous = found.end()
            pos = previous
            empty_at = pos if pos == found.start() else -1
            used += 1
        return result + string[previous:]

    def subn(self, repl, string, count=0):
        is_func = callable(repl)
        result = ''
        previous = 0
        pos = 0
        used = 0
        empty_at = -1
        while pos <= len(string) and (count == 0 or used < count):
            found = self._next(string, pos, empty_at)
            if found is None:
                break
            result += string[previous:found.start()]
            result += repl(found) if is_func else _expand(repl, found)
            previous = found.end()
            pos = previous
            empty_at = pos if pos == found.start() else -1
            used += 1
        return (result + string[previous:], used)

    def split(self, string, maxsplit=0):
        result = []
        previous = 0
        pos = 0
        used = 0
        empty_at = -1
        while pos <= len(string) and (maxsplit == 0 or used < maxsplit):
            found = self._next(string, pos, empty_at)
            if found is None:
                break
            result.append(string[previous:found.start()])
            result = [*result, *found.groups()]
            previous = found.end()
            pos = previous
            empty_at = pos if pos == found.start() else -1
            used += 1
        result.append(string[previous:])
        return result

def compile(pattern, flags=0):
    if isinstance(pattern, Pattern):
        if flags:
            raise 'ValueError: cannot process flags with a compiled pattern'
        return pattern
    return Pattern(pattern, flags)

def match(pattern, string, flags=0):
    return compile(pattern, flags).match(string)

def search(pattern, string, flags=0):
    return compile(pattern, flags).search(string)

def fullmatch(pattern, string, flags=0):
    return compile(pattern, flags).fullmatch(string)

def findall(pattern, string, flags=0):
    return compile(pattern, flags).findall(string)

def finditer(pattern, string, flags=0):
    return compile(pattern, flags).finditer(string)

def sub(pattern, repl, string, count=0, flags=0):
    return compile(pattern, flags).sub(repl, string, count)

def subn(pattern, repl, string, count=0, flags=0):
    return compile(pattern, flags).subn(repl, string, count)

def split(pattern, string, maxsplit=0, flags=0):
    return compile(pattern, flags).split(string, maxsplit)

def escape(pattern):
    result = ''
    for letter in list(pattern):
        if letter in '()[]{}?*+-|^$\\.&~# \t\n\r\v\f':
            result += '\\'
        result += letter
    return result
