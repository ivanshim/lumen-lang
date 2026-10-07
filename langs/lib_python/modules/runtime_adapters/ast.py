# Extend the expression-tree reader with simple Python statement nodes.
# Unsupported compound statements continue to report the missing tree grammar.

class Return(stmt):
    _fields = ('value',)

class Raise(stmt):
    _fields = ('exc', 'cause')

class Assert(stmt):
    _fields = ('test', 'msg')

class Pass(stmt):
    _fields = ()

class Break(stmt):
    _fields = ()

class Continue(stmt):
    _fields = ()

_ExpressionParser = _Parser

class _Parser(_ExpressionParser):
    # Parse the statement keyword and retain its complete source extent.
    def parse_stmt(self):
        token = self.peek()
        simple = {'pass': Pass, 'break': Break, 'continue': Continue}
        if token.kind != 'name' or token.text not in ('return', 'raise', 'assert', 'pass', 'break', 'continue'):
            # An unfinished compound header is invalid syntax, even when its
            # complete statement would need tree grammar not implemented here.
            if token.kind == 'name' and token.text in _STMT_WORDS:
                for last in reversed(self.toks):
                    if last.kind not in ('newline', 'end'):
                        if last.kind == 'op' and last.text == ':':
                            compile(self.source, '<unknown>', 'exec')
                        break
            return _ExpressionParser.parse_stmt(self)
        self.pop()
        node_type = simple.get(token.text)
        end = (token.erow, token.ecol)
        fields = {}
        if node_type is None:
            following = self.peek()
            empty = following.kind in ('newline', 'end') or self.at_op(';')
            if token.text == 'assert':
                if empty:
                    _syntax('invalid syntax')
                test, start, end = self.parse_expr(False)
                message = None
                if self.at_op(','):
                    self.pop()
                    message, start, end = self.parse_expr(False)
                node_type = Assert
                fields = {'test': test, 'msg': message}
            else:
                expression = None
                if not empty:
                    expression, start, end = self.parse_expr(token.text == 'return')
                if token.text == 'return':
                    node_type = Return
                    fields = {'value': expression}
                else:
                    cause = None
                    if self.at_name('from'):
                        if expression is None:
                            _syntax('invalid syntax')
                        self.pop()
                        cause, start, end = self.parse_expr(False)
                    node_type = Raise
                    fields = {'exc': expression, 'cause': cause}
        return node_type(lineno=token.srow, col_offset=token.scol,
                         end_lineno=end[0], end_col_offset=end[1], **fields)

# Expose positional structural patterns using each AST node's declared fields.
for _node_type in list(globals().values()):
    if isinstance(_node_type, type) and issubclass(_node_type, AST):
        _node_type.__match_args__ = _node_type._fields

class keyword(AST):
    _fields = ('arg', 'value')

class Starred(expr):
    _fields = ('value', 'ctx')

class Slice(expr):
    _fields = ('lower', 'upper', 'step')

class For(stmt):
    _fields = ('target', 'iter', 'body', 'orelse', 'type_comment')

class While(stmt):
    _fields = ('test', 'body', 'orelse')

class AugAssign(stmt):
    _fields = ('target', 'op', 'value')

_StatementParser = _Parser

class _Parser(_StatementParser):
    def positioned(self, node_type, start, end, **fields):
        return node_type(lineno=start[0], col_offset=start[1],
                         end_lineno=end[0], end_col_offset=end[1], **fields)

    def suite(self, header):
        self.expect_op(':')
        body = []
        if self.peek().kind != 'newline':
            while True:
                body.append(self.parse_stmt())
                if not self.at_op(';'):
                    break
                self.pop()
                if self.peek().kind in ('newline', 'end'):
                    break
            return body
        boundary = self.at
        while self.peek().kind == 'newline':
            self.pop()
        if self.peek().kind == 'end' or self.peek().scol <= header.scol:
            _syntax('expected an indented block')
        indent = self.peek().scol
        while self.peek().kind != 'end' and self.peek().scol >= indent:
            if self.peek().scol != indent:
                _syntax('unexpected indent')
            body.append(self.parse_stmt())
            if self.at_op(';'):
                self.pop()
                continue
            boundary = self.at
            while self.peek().kind == 'newline':
                self.pop()
            if self.peek().kind == 'end' or self.peek().scol < indent:
                self.at = boundary
                break
        return body

    def parse_stmt(self):
        beginning = self.peek()
        if self.at_name('for') or self.at_name('while'):
            self.pop()
            if beginning.text == 'for':
                target, start, end = self.parse_arith()
                if self.at_op(','):
                    targets = [target]
                    while self.at_op(','):
                        self.pop()
                        if self.at_name('in'):
                            break
                        item, part_start, end = self.parse_arith()
                        targets.append(item)
                    target = self.positioned(Tuple, start, end, elts=targets, ctx=Store())
                self.set_store(target)
                if not self.at_name('in'):
                    _syntax('invalid syntax')
                self.pop()
                iterable, start, end = self.parse_expr(True)
                fields = {'target': target, 'iter': iterable, 'type_comment': None}
                node_type = For
            else:
                condition, start, end = self.parse_expr(False)
                fields = {'test': condition}
                node_type = While
            body = self.suite(beginning)
            last = body[-1]
            fields['body'] = body
            fields['orelse'] = []
            return self.positioned(node_type, (beginning.srow, beginning.scol),
                                   (last.end_lineno, last.end_col_offset), **fields)
        if beginning.kind == 'name' and beginning.text in _STMT_WORDS:
            return _StatementParser.parse_stmt(self)
        saved = self.at
        target, start, end = self.parse_expr(True)
        token = self.peek()
        if token.kind == 'op' and token.text.endswith('=') and token.text[:-1] in _BINOPS:
            self.pop()
            self.set_store(target)
            value, unused, end = self.parse_expr(True)
            return self.positioned(AugAssign, start, end, target=target,
                                   op=_BINOPS[token.text[:-1]](), value=value)
        self.at = saved
        return _StatementParser.parse_stmt(self)

    def parse_postfix(self):
        node, start, end = self.parse_atom()
        while True:
            if self.at_op('('):
                self.pop()
                args = []
                keywords = []
                seen_keyword = False
                while not self.at_op(')'):
                    opening = self.peek()
                    if self.at_op('**'):
                        self.pop()
                        value, unused, end = self.parse_or()
                        keywords.append(self.positioned(keyword, (opening.srow, opening.scol), end, arg=None, value=value))
                        seen_keyword = True
                    elif self.at_op('*'):
                        self.pop()
                        value, unused, end = self.parse_or()
                        args.append(self.positioned(Starred, (opening.srow, opening.scol), end, value=value, ctx=Load()))
                    elif opening.kind == 'name' and self.toks[self.at + 1].text == '=':
                        self.pop()
                        self.pop()
                        value, unused, end = self.parse_or()
                        keywords.append(self.positioned(keyword, (opening.srow, opening.scol), end, arg=opening.text, value=value))
                        seen_keyword = True
                    else:
                        if seen_keyword:
                            _syntax('positional argument follows keyword argument')
                        argument, argument_start, end = self.parse_expr(False)
                        if self.at_name('for') or self.at_name('async'):
                            generators, end = self.comprehension_clauses()
                            argument = self.positioned(GeneratorExp, argument_start, end, elt=argument, generators=generators)
                            if args or not self.at_op(')'):
                                _syntax('Generator expression must be parenthesized')
                        args.append(argument)
                    if not self.at_op(','):
                        break
                    self.pop()
                closing = self.expect_op(')')
                end = (closing.erow, closing.ecol)
                node = self.positioned(Call, start, end, func=node, args=args, keywords=keywords)
            elif self.at_op('['):
                self.pop()
                slice_begin = self.peek()
                lower = None
                if not self.at_op(':'):
                    lower, unused, end = self.parse_expr(True)
                index = lower
                if self.at_op(':'):
                    self.pop()
                    upper = step = None
                    if not self.at_op(':') and not self.at_op(']'):
                        upper, unused, end = self.parse_expr(False)
                    if self.at_op(':'):
                        self.pop()
                        if not self.at_op(']'):
                            step, unused, end = self.parse_expr(False)
                    last = self.toks[self.at - 1]
                    index = self.positioned(Slice, (slice_begin.srow, slice_begin.scol),
                                            (last.erow, last.ecol), lower=lower, upper=upper, step=step)
                closing = self.expect_op(']')
                end = (closing.erow, closing.ecol)
                node = self.positioned(Subscript, start, end, value=node, slice=index, ctx=Load())
            elif self.at_op('.'):
                self.pop()
                attribute = self.pop()
                if attribute.kind != 'name':
                    _syntax('invalid syntax')
                end = (attribute.erow, attribute.ecol)
                node = self.positioned(Attribute, start, end, value=node, attr=attribute.text, ctx=Load())
            else:
                return node, start, end

for _node_type in list(globals().values()):
    if isinstance(_node_type, type) and issubclass(_node_type, AST):
        _node_type.__match_args__ = _node_type._fields

class IfExp(expr):
    _fields = ('test', 'body', 'orelse')

class Yield(expr):
    _fields = ('value',)

class YieldFrom(expr):
    _fields = ('value',)

class comprehension(AST):
    _fields = ('target', 'iter', 'ifs', 'is_async')
    _attributes = ()

class GeneratorExp(expr):
    _fields = ('elt', 'generators')

class ListComp(expr):
    _fields = ('elt', 'generators')

class SetComp(expr):
    _fields = ('elt', 'generators')

class DictComp(expr):
    _fields = ('key', 'value', 'generators')

_CallParser = _Parser

class _Parser(_CallParser):
    def parse_stmt(self):
        if self.at_name('yield'):
            opening = self.pop()
            start = (opening.srow, opening.scol)
            end = (opening.erow, opening.ecol)
            value = None
            node_type = Yield
            if self.at_name('from'):
                self.pop()
                node_type = YieldFrom
                value, unused, end = self.parse_expr(False)
            elif self.peek().kind not in ('newline', 'end') and not self.at_op(';'):
                value, unused, end = self.parse_expr(True)
            expression = self.positioned(node_type, start, end, value=value)
            return self.positioned(Expr, start, end, value=expression)
        return _CallParser.parse_stmt(self)

    def parse_expr(self, bare_tuple):
        node, start, end = self.parse_or()
        if self.at_name('if'):
            self.pop()
            test, unused, end = self.parse_or()
            if not self.at_name('else'):
                _syntax("expected 'else' after 'if' expression")
            self.pop()
            otherwise, unused, end = self.parse_expr(False)
            node = self.positioned(IfExp, start, end, test=test, body=node, orelse=otherwise)
        if bare_tuple and self.at_op(','):
            items = [node]
            while self.at_op(','):
                self.pop()
                if self.peek().kind in ('newline', 'end') or self.peek().text in (')', ']', '}'):
                    break
                item, unused, end = self.parse_expr(False)
                items.append(item)
            node = self.positioned(Tuple, start, end, elts=items, ctx=Load())
        return node, start, end

    def comprehension_clauses(self):
        generators = []
        end = None
        while self.at_name('for') or self.at_name('async'):
            asynchronous = 0
            if self.at_name('async'):
                self.pop()
                asynchronous = 1
            if not self.at_name('for'):
                _syntax('invalid syntax')
            self.pop()
            target, start, end = self.parse_arith()
            targets = [target]
            while self.at_op(','):
                self.pop()
                if self.at_name('in'):
                    break
                target, unused, end = self.parse_arith()
                targets.append(target)
            if len(targets) > 1:
                target = self.positioned(Tuple, start, end, elts=targets, ctx=Store())
            else:
                target = targets[0]
            self.set_store(target)
            if not self.at_name('in'):
                _syntax('invalid syntax')
            self.pop()
            iterable, unused, end = self.parse_or()
            filters = []
            while self.at_name('if'):
                self.pop()
                condition, unused, end = self.parse_or()
                filters.append(condition)
            generators.append(comprehension(target=target, iter=iterable,
                                             ifs=filters, is_async=asynchronous))
        return generators, end

    def parse_atom(self):
        saved = self.at
        opening = self.peek()
        if opening.kind != 'op' or opening.text not in ('(', '[', '{'):
            return _CallParser.parse_atom(self)
        self.pop()
        if self.peek().text in (')', ']', '}') or self.at_op('*') or self.at_op('**'):
            self.at = saved
            return _CallParser.parse_atom(self)
        first, start, end = self.parse_expr(False)
        key = None
        if opening.text == '{' and self.at_op(':'):
            self.pop()
            key = first
            first, start, end = self.parse_expr(False)
        if not self.at_name('for') and not self.at_name('async'):
            self.at = saved
            return _CallParser.parse_atom(self)
        generators, end = self.comprehension_clauses()
        closing = self.expect_op({'(': ')', '[': ']', '{': '}'}[opening.text])
        start = (opening.srow, opening.scol)
        end = (closing.erow, closing.ecol)
        if key is not None:
            result = self.positioned(DictComp, start, end, key=key, value=first, generators=generators)
        else:
            result = self.positioned({'(': GeneratorExp, '[': ListComp, '{': SetComp}[opening.text],
                                     start, end, elt=first, generators=generators)
        return result, start, end

for _node_type in list(globals().values()):
    if isinstance(_node_type, type) and issubclass(_node_type, AST):
        _node_type.__match_args__ = _node_type._fields

class arg(AST):
    _fields = ('arg', 'annotation', 'type_comment')

class arguments(AST):
    _fields = ('posonlyargs', 'args', 'vararg', 'kwonlyargs', 'kw_defaults', 'kwarg', 'defaults')
    _attributes = ()

class FunctionDef(stmt):
    _fields = ('name', 'args', 'body', 'decorator_list', 'returns', 'type_comment', 'type_params')

class AsyncFunctionDef(stmt):
    _fields = FunctionDef._fields

_ComprehensionParser = _Parser

class _Parser(_ComprehensionParser):
    def parameter(self):
        name = self.pop()
        if name.kind != 'name':
            _syntax('invalid syntax')
        start = (name.srow, name.scol)
        end = (name.erow, name.ecol)
        annotation = None
        if self.at_op(':'):
            self.pop()
            annotation, unused, end = self.parse_expr(False)
        return self.positioned(arg, start, end, arg=name.text, annotation=annotation, type_comment=None)

    def function_parameters(self):
        self.expect_op('(')
        positional = []
        posonly = []
        defaults = []
        kwonly = []
        kw_defaults = []
        vararg = kwarg = None
        keyword_only = False
        while not self.at_op(')'):
            if self.at_op('/'):
                if not positional or posonly or keyword_only:
                    _syntax('invalid syntax')
                self.pop()
                posonly, positional = positional, []
            elif self.at_op('**'):
                self.pop()
                kwarg = self.parameter()
                if self.at_op(','):
                    self.pop()
                if not self.at_op(')'):
                    _syntax('arguments cannot follow var-keyword argument')
                break
            elif self.at_op('*'):
                if keyword_only:
                    _syntax('invalid syntax')
                self.pop()
                keyword_only = True
                if not self.at_op(','):
                    vararg = self.parameter()
            else:
                parameter = self.parameter()
                default = None
                has_default = self.at_op('=')
                if has_default:
                    self.pop()
                    default, unused, end = self.parse_expr(False)
                if keyword_only:
                    kwonly.append(parameter)
                    kw_defaults.append(default)
                else:
                    if defaults and not has_default:
                        _syntax('non-default argument follows default argument')
                    positional.append(parameter)
                    if has_default:
                        defaults.append(default)
            if not self.at_op(','):
                break
            self.pop()
        self.expect_op(')')
        if keyword_only and vararg is None and not kwonly:
            _syntax('named arguments must follow bare *')
        return arguments(posonlyargs=posonly, args=positional, vararg=vararg,
                         kwonlyargs=kwonly, kw_defaults=kw_defaults,
                         kwarg=kwarg, defaults=defaults)

    def parse_stmt(self):
        asynchronous = self.at_name('async') and self.toks[self.at + 1].text == 'def'
        if not self.at_name('def') and not asynchronous:
            return _ComprehensionParser.parse_stmt(self)
        opening = self.pop()
        if asynchronous:
            self.pop()
        name = self.pop()
        if name.kind != 'name':
            _syntax('invalid syntax')
        if self.at_op('['):
            _no_tree()
        parameters = self.function_parameters()
        returns = None
        if self.at_op('->'):
            self.pop()
            returns, unused, end = self.parse_expr(False)
        body = self.suite(opening)
        last = body[-1]
        return self.positioned(AsyncFunctionDef if asynchronous else FunctionDef,
                               (opening.srow, opening.scol), (last.end_lineno, last.end_col_offset),
                               name=name.text, args=parameters, body=body, decorator_list=[],
                               returns=returns, type_comment=None, type_params=[])

for _node_type in list(globals().values()):
    if isinstance(_node_type, type) and issubclass(_node_type, AST):
        _node_type.__match_args__ = _node_type._fields

# Repeated source lines share lexical records, while every parse creates fresh
# syntax nodes. The private token reader does not alter these records.
_UncachedLexer = _lex
_LexicalCache = {}

def _lex(source, start, end, row, column, field, comments=None):
    complete = start == 0 and end == len(source) and row == 1 and column == 0 and not field
    if not complete or len(source) > 4096:
        return _UncachedLexer(source, start, end, row, column, field, comments)
    cached = _LexicalCache.get(source)
    if cached is None:
        gathered = []
        tokens = _UncachedLexer(source, start, end, row, column, field, gathered)
        cached = (tuple(tokens), tuple(gathered))
        if len(_LexicalCache) >= 16:
            _LexicalCache.clear()
        _LexicalCache[source] = cached
    if comments is not None:
        comments.extend(cached[1])
    return list(cached[0])
