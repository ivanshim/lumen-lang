# A backtracking reader over text. Lookaround, backreferences, named groups,
# inline flags, bytes, Unicode shorthand classes and locale rules are not carried by this small engine.
# Unsupported escapes and group forms raise a complaint when compiled.
I = 2
IGNORECASE = I
M = 8
MULTILINE = M
S = 16
DOTALL = S

class _Reader:
    def __init__(self, pattern):
        self.pattern = pattern
        self.i = 0
        self.groups = 0

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
                mark = self.pattern[self.i]
                self.i += 1
                low = 0
                high = -1
                if mark == '+':
                    low = 1
                elif mark == '?':
                    high = 1
                elif mark == '{':
                    low = self.number()
                    high = low
                    if self.i < len(self.pattern) and self.pattern[self.i] == ',':
                        self.i += 1
                        high = -1
                        if self.i < len(self.pattern) and self.pattern[self.i] in '0123456789':
                            high = self.number()
                    if self.i == len(self.pattern) or self.pattern[self.i] != '}':
                        raise 'ValueError: unterminated repetition'
                    self.i += 1
                    if high >= 0 and high < low:
                        raise 'ValueError: bad repetition bounds'
                greedy = True
                if self.i < len(self.pattern) and self.pattern[self.i] == '?':
                    greedy = False
                    self.i += 1
                node = ['repeat', node, low, high, greedy]
            nodes.append(node)
        return ['seq', nodes]

    def number(self):
        begin = self.i
        while self.i < len(self.pattern) and self.pattern[self.i] in '0123456789':
            self.i += 1
        if begin == self.i:
            raise 'ValueError: missing repetition bound'
        return int(self.pattern[begin:self.i])

    def escaped(self, inside=False):
        if self.i >= len(self.pattern):
            raise 'ValueError: trailing backslash'
        mark = self.pattern[self.i]
        self.i += 1
        if mark in 'dDwWsS':
            return ['kind', mark]
        if mark == 'b':
            return ['lit', '\b'] if inside else ['boundary']
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
        if mark in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789':
            raise 'NotImplementedError: this regular expression escape is not supported'
        return ['lit', mark]

    def atom(self):
        mark = self.pattern[self.i]
        self.i += 1
        if mark == '(':
            if self.i < len(self.pattern) and self.pattern[self.i] == '?':
                raise 'NotImplementedError: special regular expression groups are not supported'
            self.groups += 1
            number = self.groups
            node = self.choice()
            if self.i == len(self.pattern) or self.pattern[self.i] != ')':
                raise 'ValueError: unterminated group'
            self.i += 1
            return ['group', number, node]
        if mark == '[':
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
                    if entry[0] != 'lit' or end[0] != 'lit' or entry[1] > end[1]:
                        raise 'ValueError: bad character range'
                    entry = ['range', entry[1], end[1]]
                entries.append(entry)
            if self.i == len(self.pattern):
                raise 'ValueError: unterminated character class'
            self.i += 1
            return ['class', reverse, entries]
        if mark == '\\':
            return self.escaped()
        if mark == '.':
            return ['dot']
        if mark == '^':
            return ['start']
        if mark == '$':
            return ['end']
        if mark in '*+?{}':
            raise 'ValueError: nothing to repeat'
        return ['lit', mark]

def _word(letter):
    return letter != '' and letter in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_'

def _accept(node, letter):
    kind = node[0]
    if kind == 'lit':
        return letter == node[1]
    if kind == 'range':
        return node[1] <= letter and letter <= node[2]
    if kind == 'kind':
        mark = node[1]
        if mark in 'dD':
            answer = letter in '0123456789'
        elif mark in 'wW':
            answer = _word(letter)
        else:
            answer = letter in ' \t\n\r\v\f'
        return not answer if mark in 'DWS' else answer
    return False

def _walk(node, text, place, captures):
    kind = node[0]
    if kind == 'seq':
        states = [[place, captures]]
        for part in node[1]:
            next_states = []
            for state in states:
                next_states = [*next_states, *_walk(part, text, state[0], state[1])]
            states = next_states
        return states
    if kind == 'or':
        states = []
        for arm in node[1]:
            states = [*states, *_walk(arm, text, place, captures)]
        return states
    if kind == 'group':
        states = []
        for state in _walk(node[2], text, place, captures):
            found = __copy_value(state[1], False)
            found[node[1]] = [place, state[0]]
            states.append([state[0], found])
        return states
    if kind == 'repeat':
        return _repeat(node, text, place, captures, 0)
    if kind == 'start':
        return [[place, captures]] if place == 0 else []
    if kind == 'end':
        return [[place, captures]] if place == len(text) or (place == len(text) - 1 and text[place] == '\n') else []
    if kind == 'boundary':
        before = _word(text[place - 1]) if place > 0 else False
        after = _word(text[place]) if place < len(text) else False
        return [[place, captures]] if before != after else []
    if place >= len(text):
        return []
    letter = text[place]
    if kind == 'dot':
        answer = letter != '\n'
    elif kind == 'class':
        answer = False
        for entry in node[2]:
            if _accept(entry, letter):
                answer = True
        if node[1]:
            answer = not answer
    else:
        answer = _accept(node, letter)
    return [[place + 1, captures]] if answer else []

def _repeat(node, text, place, captures, count):
    states = []
    if count >= node[2] and not node[4]:
        states.append([place, captures])
    if node[3] < 0 or count < node[3]:
        for state in _walk(node[1], text, place, captures):
            if state[0] != place:
                states = [*states, *_repeat(node, text, state[0], state[1], count + 1)]
            elif count + 1 >= node[2]:
                states.append(state)
            elif count < node[2]:
                states = [*states, *_repeat(node, text, place, state[1], count + 1)]
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

    def group(self, *numbers):
        if len(numbers) == 0:
            numbers = [0]
        values = []
        for number in numbers:
            if number < 0 or number > self.re.groups:
                raise 'IndexError: no such group'
            if number in self.captures:
                span = self.captures[number]
                values.append(self.string[span[0]:span[1]])
            else:
                values.append(None)
        return values[0] if len(values) == 1 else values

    def start(self, group=0):
        return self.captures[group][0] if group in self.captures else -1

    def end(self, group=0):
        return self.captures[group][1] if group in self.captures else -1

    def span(self, group=0):
        return (self.start(group), self.end(group))

    def groups(self, default=None):
        values = []
        for i in range(1, self.re.groups + 1):
            value = self.group(i)
            values.append(default if value is None else value)
        return values

class Pattern:
    def __init__(self, pattern, flags=0):
        if flags != 0:
            raise 'NotImplementedError: regular expression flags are not supported'
        reader = _Reader(pattern)
        self.tree = reader.choice()
        if reader.i != len(pattern):
            raise 'ValueError: unbalanced parenthesis'
        self.groups = reader.groups
        self.pattern = pattern
        self.flags = flags

    def _check_text(self, string):
        shorthand = False
        for mark in ['\\w', '\\W', '\\d', '\\D', '\\s', '\\S', '\\b']:
            if mark in self.pattern:
                shorthand = True
        if shorthand:
            for letter in list(string):
                if ord(letter) > 127:
                    raise 'NotImplementedError: Unicode shorthand classes are not supported'

    def match(self, string, pos=0, endpos=None):
        self._check_text(string)
        if endpos is not None:
            string = string[:endpos]
        states = _walk(self.tree, string, pos, {})
        return Match(self, string, pos, states[0]) if len(states) else None

    def fullmatch(self, string, pos=0, endpos=None):
        self._check_text(string)
        if endpos is not None:
            string = string[:endpos]
        for state in _walk(self.tree, string, pos, {}):
            if state[0] == len(string):
                return Match(self, string, pos, state)
        return None

    def search(self, string, pos=0, endpos=None):
        if endpos is not None:
            string = string[:endpos]
        while pos <= len(string):
            found = self.match(string, pos)
            if found is not None:
                return found
            pos += 1
        return None

    def _next(self, string, pos, empty_at):
        self._check_text(string)
        while pos <= len(string):
            for state in _walk(self.tree, string, pos, {}):
                if pos != empty_at or state[0] != pos:
                    return Match(self, string, pos, state)
            pos += 1
        return None

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
        if type(repl) != type('') or '\\' in repl:
            raise 'NotImplementedError: replacement functions and escapes are not supported'
        result = ''
        previous = 0
        pos = 0
        used = 0
        empty_at = -1
        while pos <= len(string) and (count == 0 or used < count):
            found = self._next(string, pos, empty_at)
            if found is None:
                break
            result += string[previous:found.start()] + repl
            previous = found.end()
            pos = previous
            empty_at = pos if pos == found.start() else -1
            used += 1
        return result + string[previous:]

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

def sub(pattern, repl, string, count=0, flags=0):
    return compile(pattern, flags).sub(repl, string, count)

def split(pattern, string, maxsplit=0, flags=0):
    return compile(pattern, flags).split(string, maxsplit)

def escape(pattern):
    result = ''
    for letter in list(pattern):
        if letter in '()[]{}?*+-|^$\\.&~# \t\n\r\v\f':
            result += '\\'
        result += letter
    return result
