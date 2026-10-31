# The CLI's interactive input loop. Compilation state and the user namespace
# survive each command; this is also used after a script or -c with -i.
def _lumen_interact(source, filename, namespace, quiet=False, version="3.14.8"):
    def _lumen_word(text, at):
        # The identifier beginning at `at`, or nothing.
        end = at
        while end < len(text) and (text[end].isalpha() or text[end] == '_'):
            end += 1
        return text[at:end]


    def _lumen_advance(state, text):
        # Carry the reading of one more line over the state: the mark of a
        # literal/field stack, the count of statement brackets still open, the last
        # sign written outside any string, the statement's first word,
        # whether it holds an assignment or a statement separator, whether
        # it began with a decorator, whether the line was blank, and
        # whether it ended by joining the next line. Only the line just
        # added is walked, so a long session costs no more than its text.
        frames = state[11]
        logical_start = not frames and state[1] == 0 and not state[8]
        state[8] = False
        at = 0
        end = len(text)
        while at < end:
            mark = text[at]
            frame = frames[-1] if frames else None
            if frame is not None and frame[0] in ('string', 'format'):
                if frame[0] == 'string':
                    delimiter = frame[1]
                    if mark == '\\':
                        if text[at + 1:] in ('', '\n', '\r\n'):
                            state[8] = True
                        # Backslashes do not escape replacement-field braces.
                        if frame[2] and text[at + 1:at + 2] in ('{', '}'):
                            at += 1
                        elif frame[2] and not frame[3] and text[at + 1:at + 3] == 'N{':
                            closing = text.find('}', at + 3)
                            at = closing + 1 if closing >= 0 else end
                        else:
                            at += 3 if text[at + 1:at + 3] == '\r\n' else 2
                        continue
                    if text[at:at + len(delimiter)] == delimiter:
                        frames.pop()
                        if not frames:
                            state[2] = mark
                        at += len(delimiter)
                        continue
                    if len(delimiter) == 1 and mark == '\n':
                        frames.pop()
                        at += 1
                        continue
                    formatted = frame[2]
                else:
                    formatted = True
                    if mark == '\\' and not frame[1] and text[at + 1:at + 3] == 'N{':
                        closing = text.find('}', at + 3)
                        at = closing + 1 if closing >= 0 else end
                        continue
                if formatted and mark == '{':
                    if frame[0] == 'string' and text[at + 1:at + 2] == '{':
                        at += 2
                        continue
                    frames.append(['field', 0, frame[3] if frame[0] == 'string' else frame[1]])
                elif formatted and mark == '}':
                    if frame[0] == 'format':
                        frames.pop()
                    elif text[at + 1:at + 2] == '}':
                        at += 2
                        continue
                at += 1
                continue
            if mark == '#':
                break
            if mark == '\\':
                if text[at + 1:] in ('', '\n', '\r\n'):
                    state[8] = True
                at += 2
                continue
            if mark in ("'", '"'):
                start = at
                while start > 0 and (text[start - 1].isalnum() or text[start - 1] == '_'):
                    start -= 1
                prefix = text[start:at].lower()
                formatted = prefix in ('f', 'fr', 'rf', 't', 'tr', 'rt')
                delimiter = mark * 3 if text[at:at + 3] == mark * 3 else mark
                frames.append(['string', delimiter, formatted, 'r' in prefix])
                at += len(delimiter)
                continue
            if frame is not None:
                # Field brackets and nested literals do not affect the
                # surrounding statement's depth or assignment classification.
                if mark == '}' and frame[1] == 0:
                    frames.pop()
                elif mark == ':' and frame[1] == 0:
                    frames[-1] = ['format', frame[2]]
                elif mark in '([{':
                    frame[1] += 1
                elif mark in ')]}':
                    frame[1] -= 1
                at += 1
                continue
            if mark in '([{':
                state[1] = state[1] + 1
            elif mark in ')]}':
                if state[1] > 0:
                    state[1] = state[1] - 1
            elif mark == '=' and state[1] == 0:
                after = text[at + 1] if at + 1 < end else ''
                before = text[at - 1] if at > 0 else ''
                if after != '=' and before not in '=!<>:':
                    state[4] = True
            elif mark == ';' and state[1] == 0:
                state[5] = True
            if state[1] == 0 and mark not in ' \t\r\n':
                state[2] = mark
            at += 1
        state[0] = bool(frames)
        state[7] = text.strip() == ""
        lead = text.lstrip()
        if logical_start and state[6] and _lumen_word(lead, 0) in ('def', 'class', 'async'):
            state[6] = False
            state[3] = None
        if state[3] is None and lead and lead[0] != '#':
            state[3] = _lumen_word(text, len(text) - len(lead)) or lead[0]
            if lead[0] == '@':
                state[6] = True


    def _lumen_fresh():
        # literal pending, depth, last sign, first word, assignment, separator,
        # decorator, blank, joined, compound, header indent, literal/field stack.
        return [False, 0, None, None, False, False, False, False, False, False, 0, []]


    import codeop
    import sys
    if not quiet:
        sys.stderr.write("Python " + version + " (lumen-lang)\n")
        sys.stderr.flush()
    compiler = codeop.CommandCompiler()
    def report():
        import traceback
        traceback.print_exc()
    if source:
        try:
            initial = compile(source, filename, 'exec')
            import __future__
            for name in __future__.all_feature_names:
                compiler.compiler.flags |= initial.co_flags & getattr(__future__, name).compiler_flag
            exec(initial, namespace)
        except Exception:
            report()
    if not hasattr(sys, 'ps1'):
        sys.ps1 = '>>> '
    if not hasattr(sys, 'ps2'):
        sys.ps2 = '... '
    pending = []
    state = _lumen_fresh()
    while True:
        try:
            sys.stderr.write(sys.ps2 if pending else sys.ps1)
            sys.stderr.flush()
            line = sys.stdin.readline()
            if not line:
                if pending:
                    # At EOF, finish the buffered input without incomplete-input flags.
                    command = compiler.compiler('\n'.join(pending), '<stdin>', 'single', incomplete_input=False)
                    pending = []
                    exec(command, namespace)
                sys.stderr.write("\n")
                sys.stderr.flush()
                return
            pending.append(line.rstrip('\n'))
            _lumen_advance(state, line)
            if state[0] or state[1] > 0 or state[8] or state[6]:
                continue
            if state[9]:
                if not state[7]:
                    lead = line.lstrip(' \t')
                    if len(line) - len(lead) > state[10] or _lumen_word(lead, 0) in ('else', 'elif', 'except', 'finally', 'case'):
                        continue
            elif state[2] == ':':
                state[9] = True
                state[10] = len(line) - len(line.lstrip(' \t'))
                continue
            text = '\n'.join(pending)
            # Compile a complete plain assignment once, retaining future flags.
            if state[4] and not state[5] and not state[9] and state[3] != 'lambda':
                command = compiler.compiler(text, '<stdin>', 'exec')
            else:
                command = compiler(text, '<stdin>', 'single')
            if command is None:
                continue
            pending = []
            state = _lumen_fresh()
            exec(command, namespace)
        except KeyboardInterrupt:
            state = _lumen_fresh()
            pending = []
            sys.stderr.write('\nKeyboardInterrupt\n')
        except Exception:
            state = _lumen_fresh()
            pending = []
            report()
