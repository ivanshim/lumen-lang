"""A tracing debugger with stepping and live frame inspection."""
import cmd
import linecache
import sys


class BdbQuit(Exception):
    """Stop a debugged computation at the user's request."""


class Pdb(cmd.Cmd):
    prompt = '(Pdb) '

    def __init__(self, completekey='tab', stdin=None, stdout=None, *,
                 nosigint=False):
        cmd.Cmd.__init__(self, completekey, stdin, stdout)
        self.nosigint = nosigint
        self.reset()

    def reset(self):
        self.botframe = None
        self.stopframe = None
        self.returnframe = None
        self.quitting = False
        self.running = False
        self.stack = []
        self.curindex = 0
        self.cmdqueue = []

    def set_trace(self, frame=None, *, commands=None):
        if frame is None:
            frame = sys._getframe().f_back
        self.reset()
        self.running = True
        current = frame
        while current is not None:
            current.f_trace = self.trace_dispatch
            self.botframe = current
            current = current.f_back
        self.cmdqueue = list(commands or ())
        self.interaction(frame, None)
        if self.running:
            sys.settrace(self.trace_dispatch)

    def set_continue(self):
        self.running = False
        sys.settrace(None)

    def trace_dispatch(self, frame, event, arg):
        if self.quitting:
            raise BdbQuit
        if not self.running:
            return None
        if self.botframe is None and event == 'call':
            self.botframe = frame.f_back
            return self.trace_dispatch
        if frame is self.botframe:
            return None
        if self.stopframe is not None and frame is not self.stopframe and frame is not self.returnframe:
            return self.trace_dispatch
        if event == 'call':
            self.message('--Call--')
        elif event == 'return':
            self.message('--Return--')
        elif event == 'exception':
            self.message(type(arg[1]).__name__ + ': ' + str(arg[1]))
        elif event != 'line':
            return self.trace_dispatch
        self.interaction(frame, (event, arg))
        if self.quitting:
            raise BdbQuit
        return self.trace_dispatch if self.running else None

    def interaction(self, frame, event):
        self.stack = []
        cursor = frame
        while cursor is not None and cursor is not self.botframe:
            self.stack.append((cursor, cursor.f_lineno))
            cursor = cursor.f_back
        self.stack.reverse()
        self.curindex = len(self.stack) - 1
        self.returning = frame if event and event[0] == 'return' else None
        self.returnvalue = event[1] if self.returning is not None else None
        self.print_stack_entry()
        saved = sys.stdout
        sys.stdout = self.stdout
        try:
            self.cmdloop()
        finally:
            sys.stdout = saved

    def message(self, text):
        self.stdout.write(str(text) + '\n')

    def error(self, text):
        self.message('*** ' + str(text))

    def print_stack_entry(self):
        frame, lineno = self.stack[self.curindex]
        code = frame.f_code
        suffix = '->' + repr(self.returnvalue) if frame is self.returning else ''
        self.message('> %s(%s)%s()%s' % (code.co_filename, lineno, code.co_name, suffix))
        source = linecache.getline(code.co_filename, lineno, frame.f_globals).strip()
        if source:
            self.message('-> ' + source)

    def default(self, line):
        if line.startswith('!'):
            line = line[1:]
        frame = self.stack[self.curindex][0]
        try:
            exec(compile(line + '\n', '<stdin>', 'single'), frame.f_globals, frame.f_locals)
        except Exception as exc:
            self.error(type(exc).__name__ + ': ' + str(exc))

    def do_p(self, expression):
        frame = self.stack[self.curindex][0]
        try:
            self.message(repr(eval(expression, frame.f_globals, frame.f_locals)))
        except Exception as exc:
            self.error(type(exc).__name__ + ': ' + str(exc))

    def do_step(self, arg):
        self.stopframe = self.returnframe = None
        return True

    do_s = do_step

    def do_next(self, arg):
        frame = self.stack[self.curindex][0]
        self.stopframe = frame.f_back if frame is self.returning else frame
        self.returnframe = None
        return True

    do_n = do_next

    def do_return(self, arg):
        frame = self.stack[self.curindex][0]
        self.stopframe, self.returnframe = frame.f_back, frame
        return True

    do_r = do_return

    def do_continue(self, arg):
        self.set_continue()
        return True

    do_c = do_cont = do_continue

    def do_quit(self, arg):
        self.quitting = True
        self.set_continue()
        return True

    do_q = do_exit = do_EOF = do_quit

    def do_up(self, arg):
        if self.curindex == 0:
            self.error('Oldest frame')
        else:
            self.curindex -= 1
            self.print_stack_entry()

    do_u = do_up

    def do_down(self, arg):
        if self.curindex == len(self.stack) - 1:
            self.error('Newest frame')
        else:
            self.curindex += 1
            self.print_stack_entry()

    do_d = do_down

    def do_list(self, arg):
        frame, lineno = self.stack[self.curindex]
        lines = linecache.getlines(frame.f_code.co_filename, frame.f_globals)
        first = max(1, lineno - 5)
        if arg:
            try:
                first = max(1, int(arg) - 5)
            except ValueError:
                self.error('Invalid line number')
                return
        last = first + 10
        for number in range(first, min(last, len(lines)) + 1):
            marker = '->' if number == lineno else '  '
            self.message('%3d %s %s' % (number, marker, lines[number - 1].rstrip()))
        if last >= len(lines):
            self.message('[EOF]')

    do_l = do_list

    def run(self, statement, globals=None, locals=None):
        if globals is None:
            import __main__
            globals = __main__.__dict__
        if locals is None:
            locals = globals
        self.reset()
        self.running = True
        sys.settrace(self.trace_dispatch)
        try:
            exec(statement, globals, locals)
        except BdbQuit:
            pass
        finally:
            self.set_continue()


def run(statement, globals=None, locals=None):
    Pdb().run(statement, globals, locals)


def set_trace(*, header=None, commands=None):
    debugger = Pdb()
    if header is not None:
        debugger.message(header)
    debugger.set_trace(sys._getframe().f_back, commands=commands)
