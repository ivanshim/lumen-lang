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
        # Clause headers have no standalone statement form.
        if token.kind == 'name' and token.text in ('except', 'elif', 'else', 'finally', 'case'):
            compile(self.source, '<unknown>', 'exec')
            _syntax('invalid syntax')
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

class Lambda(expr):
    _fields = ('args', 'body')

class FunctionDef(stmt):
    _fields = ('name', 'args', 'body', 'decorator_list', 'returns', 'type_comment', 'type_params')

class AsyncFunctionDef(stmt):
    _fields = FunctionDef._fields

_ComprehensionParser = _Parser

class _Parser(_ComprehensionParser):
    # Parse a parameter; lambda parameters have no annotations.
    def parameter(self, annotations=True):
        name = self.pop()
        if name.kind != 'name':
            _syntax('invalid syntax')
        start = (name.srow, name.scol)
        end = (name.erow, name.ecol)
        annotation = None
        if annotations and self.at_op(':'):
            self.pop()
            annotation, unused, end = self.parse_expr(False)
        return self.positioned(arg, start, end, arg=name.text, annotation=annotation, type_comment=None)

    # Read the shared function/lambda parameter grammar.
    def function_parameters(self, terminator=')', parenthesized=True, annotations=True):
        if parenthesized:
            self.expect_op('(')
        positional = []
        posonly = []
        defaults = []
        kwonly = []
        kw_defaults = []
        vararg = kwarg = None
        keyword_only = False
        while not self.at_op(terminator):
            if self.at_op('/'):
                if not positional or posonly or keyword_only:
                    _syntax('invalid syntax')
                self.pop()
                posonly, positional = positional, []
            elif self.at_op('**'):
                self.pop()
                kwarg = self.parameter(annotations)
                if self.at_op(','):
                    self.pop()
                if not self.at_op(terminator):
                    _syntax('arguments cannot follow var-keyword argument')
                break
            elif self.at_op('*'):
                if keyword_only:
                    _syntax('invalid syntax')
                self.pop()
                keyword_only = True
                if not self.at_op(','):
                    vararg = self.parameter(annotations)
            else:
                parameter = self.parameter(annotations)
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
        self.expect_op(terminator)
        if keyword_only and vararg is None and not kwonly:
            _syntax('named arguments must follow bare *')
        return arguments(posonlyargs=posonly, args=positional, vararg=vararg,
                         kwonlyargs=kwonly, kw_defaults=kw_defaults,
                         kwarg=kwarg, defaults=defaults)

    # Accept lambda expressions in parenthesized and call-argument entries.
    def parse_or(self):
        if self.at_name('lambda'):
            return self.parse_expr(False)
        return _ComprehensionParser.parse_or(self)

    # Parse lambdas at expression precedence and preserve their full span.
    def parse_expr(self, bare_tuple):
        if not self.at_name('lambda'):
            return _ComprehensionParser.parse_expr(self, bare_tuple)
        opening = self.pop()
        parameters = self.function_parameters(':', False, False)
        body, unused, end = self.parse_expr(False)
        start = (opening.srow, opening.scol)
        return self.positioned(Lambda, start, end, args=parameters, body=body), start, end

    def parse_stmt(self):
        if self.at_name('lambda'):
            value, start, end = self.parse_expr(False)
            return self.positioned(Expr, start, end, value=value)
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

# Field cardinalities from CPython Parser/Python.asdl at v3.14.8; PSF License.
_DumpCardinality = {'Module': ((), ('body', 'type_ignores')), 'Interactive': ((), ('body',)), 'Expression': ((), ()), 'FunctionType': ((), ('argtypes',)), 'FunctionDef': (('returns', 'type_comment'), ('body', 'decorator_list', 'type_params')), 'AsyncFunctionDef': (('returns', 'type_comment'), ('body', 'decorator_list', 'type_params')), 'ClassDef': ((), ('bases', 'keywords', 'body', 'decorator_list', 'type_params')), 'Return': (('value',), ()), 'Delete': ((), ('targets',)), 'Assign': (('type_comment',), ('targets',)), 'TypeAlias': ((), ('type_params',)), 'AugAssign': ((), ()), 'AnnAssign': (('value',), ()), 'For': (('type_comment',), ('body', 'orelse')), 'AsyncFor': (('type_comment',), ('body', 'orelse')), 'While': ((), ('body', 'orelse')), 'If': ((), ('body', 'orelse')), 'With': (('type_comment',), ('items', 'body')), 'AsyncWith': (('type_comment',), ('items', 'body')), 'Match': ((), ('cases',)), 'Raise': (('exc', 'cause'), ()), 'Try': ((), ('body', 'handlers', 'orelse', 'finalbody')), 'TryStar': ((), ('body', 'handlers', 'orelse', 'finalbody')), 'Assert': (('msg',), ()), 'Import': ((), ('names',)), 'ImportFrom': (('module', 'level'), ('names',)), 'Global': ((), ('names',)), 'Nonlocal': ((), ('names',)), 'Expr': ((), ()), 'BoolOp': ((), ('values',)), 'NamedExpr': ((), ()), 'BinOp': ((), ()), 'UnaryOp': ((), ()), 'Lambda': ((), ()), 'IfExp': ((), ()), 'Dict': ((), ('keys', 'values')), 'Set': ((), ('elts',)), 'ListComp': ((), ('generators',)), 'SetComp': ((), ('generators',)), 'DictComp': ((), ('generators',)), 'GeneratorExp': ((), ('generators',)), 'Await': ((), ()), 'Yield': (('value',), ()), 'YieldFrom': ((), ()), 'Compare': ((), ('ops', 'comparators')), 'Call': ((), ('args', 'keywords')), 'FormattedValue': (('format_spec',), ()), 'Interpolation': (('format_spec',), ()), 'JoinedStr': ((), ('values',)), 'TemplateStr': ((), ('values',)), 'Constant': (('kind',), ()), 'Attribute': ((), ()), 'Subscript': ((), ()), 'Starred': ((), ()), 'Name': ((), ()), 'List': ((), ('elts',)), 'Tuple': ((), ('elts',)), 'Slice': (('lower', 'upper', 'step'), ()), 'comprehension': ((), ('ifs',)), 'ExceptHandler': (('type', 'name'), ('body',)), 'arguments': (('vararg', 'kwarg'), ('posonlyargs', 'args', 'kwonlyargs', 'kw_defaults', 'defaults')), 'arg': (('annotation', 'type_comment'), ()), 'keyword': (('arg',), ()), 'alias': (('asname',), ()), 'withitem': (('optional_vars',), ()), 'match_case': (('guard',), ('body',)), 'MatchValue': ((), ()), 'MatchSingleton': ((), ()), 'MatchSequence': ((), ('patterns',)), 'MatchMapping': (('rest',), ('keys', 'patterns')), 'MatchClass': ((), ('patterns', 'kwd_attrs', 'kwd_patterns')), 'MatchStar': (('name',), ()), 'MatchAs': (('pattern', 'name'), ()), 'MatchOr': ((), ('patterns',)), 'TypeIgnore': ((), ()), 'TypeVar': (('bound', 'default_value'), ()), 'ParamSpec': (('default_value',), ()), 'TypeVarTuple': (('default_value',), ())}

def dump(node, annotate_fields=True, include_attributes=False, *, indent=None, show_empty=False):
    if not isinstance(node, AST):
        raise TypeError('expected AST, got %r' % type(node).__name__)
    spacing = indent if isinstance(indent, str) or indent is None else ' ' * indent

    def render(value, depth):
        prefix = '' if spacing is None else '\n' + spacing * (depth + 1)
        separator = ', ' if spacing is None else ',' + prefix
        if isinstance(value, AST):
            typename = type(value).__name__
            optional, sequences = _DumpCardinality.get(typename, ((), ()))
            parts = []
            pending = []
            keywords = annotate_fields
            simple = True
            for field in value._fields:
                if not hasattr(value, field):
                    keywords = True
                    continue
                item = getattr(value, field)
                if item is None and field in optional:
                    keywords = True
                    continue
                if not show_empty:
                    if field in sequences and item == []:
                        if not keywords:
                            pending.append('[]')
                        continue
                    if not keywords:
                        parts.extend(pending)
                        pending = []
                text, atomic = render(item, depth + 1)
                simple = simple and atomic
                parts.append(field + '=' + text if keywords else text)
            if include_attributes:
                for field in value._attributes:
                    if not hasattr(value, field):
                        continue
                    item = getattr(value, field)
                    if item is None:
                        continue
                    text, atomic = render(item, depth + 1)
                    simple = simple and atomic
                    parts.append(field + '=' + text)
            if simple and len(parts) <= 3:
                return typename + '(' + ', '.join(parts) + ')', not parts
            return typename + '(' + prefix + separator.join(parts) + ')', False
        if isinstance(value, list):
            if not value:
                return '[]', True
            return '[' + prefix + separator.join(render(item, depth + 1)[0] for item in value) + ']', False
        return repr(value), True

    return render(node, 0)[0]

class TypeIgnore(AST):
    _fields = ('lineno', 'tag')
    _attributes = ()
    __match_args__ = _fields

_CharacterParse = parse

def parse(source, filename='<unknown>', mode='exec', *, type_comments=False,
          feature_version=None, optimize=-1, fold_debug=False):
    tree = _CharacterParse(source, filename, mode, type_comments=type_comments,
                           feature_version=feature_version, optimize=optimize, fold_debug=fold_debug)
    wide_columns = len(source.encode('utf-8')) != len(source)
    if not type_comments and not wide_columns:
        return tree
    lines = source.split('\n')
    nodes = []
    todo = [tree]
    while todo:
        node = todo.pop()
        nodes.append(node)
        for field in node._fields:
            value = getattr(node, field, None)
            if isinstance(value, AST):
                todo.append(value)
            elif isinstance(value, list):
                todo.extend(item for item in value if isinstance(item, AST))
    if type_comments and mode == 'exec':
        comments = []
        _lex(source, 0, len(source), 1, 0, False, comments)
        for row, column, text in comments:
            payload = _type_comment_payload(text)
            if payload is None:
                label = text[1:].lstrip()
                if label.startswith('type:'):
                    label = label[5:].lstrip()
                    if label.startswith('ignore') and (len(label) == 6 or not label[6].isalnum()):
                        tree.type_ignores.append(TypeIgnore(lineno=row, tag=label[6:]))
                continue
            candidates = [node for node in nodes if isinstance(node, stmt)
                          and 'type_comment' in node._fields and node.lineno <= row <= node.end_lineno]
            if not candidates:
                _syntax('invalid syntax')
            statement = min(candidates, key=lambda node: (node.end_lineno - node.lineno, -node.col_offset))
            statement.type_comment = payload
            if isinstance(statement, Assign):
                statement.end_lineno = row
                statement.end_col_offset = column + len(text)
    if not wide_columns:
        return tree
    # Syntax trees use UTF-8 byte columns; the reader's source slices use characters.
    byte_columns = {}
    for node in nodes:
        for line_field, column_field in (('lineno', 'col_offset'), ('end_lineno', 'end_col_offset')):
            row = getattr(node, line_field, None)
            column = getattr(node, column_field, None)
            if row is not None and column is not None:
                key = (row, column)
                if key not in byte_columns:
                    byte_columns[key] = len(lines[row - 1][:column].encode('utf-8'))
                setattr(node, column_field, byte_columns[key])
    return tree

class If(stmt):
    _fields = ('test', 'body', 'orelse')
    __match_args__ = _fields

_FunctionStatementParser = _Parser

class _Parser(_FunctionStatementParser):
    # Read an if/elif suite and its optional continuation at the same indent.
    def conditional_statement(self):
        opening = self.pop()
        condition, unused, end = self.parse_expr(False)
        body = self.suite(opening)
        saved = self.at
        while self.peek().kind == 'newline':
            self.pop()
        otherwise = []
        if self.peek().scol == opening.scol and self.at_name('elif'):
            otherwise = [self.conditional_statement()]
        elif self.peek().scol == opening.scol and self.at_name('else'):
            marker = self.pop()
            otherwise = self.suite(marker)
        else:
            self.at = saved
        last = (otherwise or body)[-1]
        return self.positioned(If, (opening.srow, opening.scol),
                               (last.end_lineno, last.end_col_offset),
                               test=condition, body=body, orelse=otherwise)

    # Route complete if statements through their clause parser.
    def parse_stmt(self):
        if self.at_name('if'):
            return self.conditional_statement()
        return _FunctionStatementParser.parse_stmt(self)

_BitwiseParser = _Parser
_BitwiseTiers = (('|',), ('^',), ('&',), ('<<', '>>'))
_BitwiseSigns = ('|', '^', '&', '<<', '>>')

class _Parser(_BitwiseParser):
    def parse_arith(self):
        # What follows the plain arithmetic is nearly always no bitwise
        # sign, which no tier then takes; the tiers are walked, loosest
        # last, only when one stands there.
        node, start, end = _ExpressionParser.parse_arith(self)
        token = self.toks[self.at]
        if token.kind != 'op' or token.text not in _BitwiseSigns:
            return node, start, end
        for level in (3, 2, 1, 0):
            tier = _BitwiseTiers[level]
            while True:
                token = self.toks[self.at]
                if token.kind != 'op' or token.text not in tier:
                    break
                sign = self.pop().text
                right, part_start, end = self.bitwise_expression(level + 1)
                node = BinOp(left=node, op=_BINOPS[sign](), right=right,
                             lineno=start[0], col_offset=start[1],
                             end_lineno=end[0], end_col_offset=end[1])
        return node, start, end

    def bitwise_expression(self, level):
        tiers = (('|',), ('^',), ('&',), ('<<', '>>'))
        if level == len(tiers):
            return _ExpressionParser.parse_arith(self)
        node, start, end = self.bitwise_expression(level + 1)
        while self.peek().kind == 'op' and self.peek().text in tiers[level]:
            sign = self.pop().text
            right, part_start, end = self.bitwise_expression(level + 1)
            node = BinOp(left=node, op=_BINOPS[sign](), right=right,
                         lineno=start[0], col_offset=start[1],
                         end_lineno=end[0], end_col_offset=end[1])
        return node, start, end

class ClassDef(stmt):
    _fields = ('name', 'bases', 'keywords', 'body', 'decorator_list', 'type_params')

_ClassStatementParser = _Parser
class _Parser(_ClassStatementParser):
    def parse_stmt(self):
        if not self.at_name('class'):
            return _ClassStatementParser.parse_stmt(self)
        opening = self.pop()
        name = self.pop()
        if name.kind != 'name':
            _syntax('invalid syntax')
        bases = []
        options = []
        if self.at_op('('):
            self.pop()
            while not self.at_op(')'):
                if self.peek().kind == 'name' and self.toks[self.at + 1].text == '=':
                    key = self.pop()
                    self.pop()
                    value, start, end = self.parse_expr(False)
                    options.append(self.positioned(keyword, (key.srow, key.scol), end, arg=key.text, value=value))
                else:
                    value, start, end = self.parse_expr(False)
                    bases.append(value)
                if not self.at_op(','):
                    break
                self.pop()
            self.expect_op(')')
        body = self.suite(opening)
        last = body[-1]
        return self.positioned(ClassDef, (opening.srow, opening.scol),
                               (last.end_lineno, last.end_col_offset),
                               name=name.text, bases=bases, keywords=options,
                               body=body, decorator_list=[], type_params=[])

class Delete(stmt):
    _fields = ('targets',)

_DeleteStatementParser = _Parser
class _Parser(_DeleteStatementParser):
    def deleted_target(self, node):
        if isinstance(node, (Name, Attribute, Subscript)):
            node.ctx = Del()
        elif isinstance(node, (Tuple, List)):
            node.ctx = Del()
            for item in node.elts:
                self.deleted_target(item)
        else:
            _syntax('cannot delete expression')

    def parse_stmt(self):
        if not self.at_name('del'):
            return _DeleteStatementParser.parse_stmt(self)
        first = self.pop()
        targets = []
        while True:
            value, start, end = self.parse_expr(False)
            self.deleted_target(value)
            targets.append(value)
            if not self.at_op(','):
                break
            self.pop()
        return self.positioned(Delete, (first.srow, first.scol), end, targets=targets)

# Inline and semicolon suites admit simple statements only.
_InlineStatementParser = _Parser
class _Parser(_InlineStatementParser):
    def parse_simple_stmt(self):
        token = self.peek()
        if token.kind == 'name' and token.text in ('def', 'class', 'if', 'for', 'while', 'try', 'with', 'async', 'match'):
            _syntax('invalid syntax')
        if token.kind == 'name' and token.text in ('return', 'raise', 'assert', 'pass', 'break', 'continue', 'yield', 'del', 'lambda'):
            return _InlineStatementParser.parse_stmt(self)
        return _ExpressionParser.parse_simple_stmt(self)

# The optimized syntax-tree flag folds the compile-time debugging name.
def _fold_debug_names(node, enabled):
    if isinstance(node, Name) and node.id == '__debug__' and isinstance(node.ctx, Load):
        return copy_location(Constant(value=enabled, kind=None), node)
    if isinstance(node, AST):
        for field in node._fields:
            value = getattr(node, field, None)
            if isinstance(value, AST):
                setattr(node, field, _fold_debug_names(value, enabled))
            elif isinstance(value, list):
                for index, item in enumerate(value):
                    if isinstance(item, AST):
                        value[index] = _fold_debug_names(item, enabled)
    return node

def _parse_for_compile(source, filename, mode, type_comments, optimize, optimized=False):
    tree = parse(source, filename, mode, type_comments=bool(type_comments), optimize=optimize)
    if optimized:
        tree = _fold_debug_names(tree, optimize < 1)
    return tree

# Retain the source coordinates supported by both syntax nodes.
def copy_location(new_node, old_node):
    for attribute in ('lineno', 'col_offset', 'end_lineno', 'end_col_offset'):
        if attribute in old_node._attributes and attribute in new_node._attributes:
            value = getattr(old_node, attribute, None)
            if value is not None or attribute in ('end_lineno', 'end_col_offset'):
                setattr(new_node, attribute, value)
    return new_node

# Compound exception statements retain each handler and its own extent.
class Try(stmt):
    _fields = ('body', 'handlers', 'orelse', 'finalbody')
class TryStar(stmt):
    _fields = Try._fields
class ExceptHandler(AST):
    _fields = ('type', 'name', 'body')
    _attributes = ('lineno', 'col_offset', 'end_lineno', 'end_col_offset')

_ExceptionStatementParser = _Parser
class _Parser(_ExceptionStatementParser):
    def parse_stmt(self):
        if not self.at_name('try'):
            return _ExceptionStatementParser.parse_stmt(self)
        opening = self.pop()
        body = self.suite(opening)
        handlers = []
        otherwise = []
        finalbody = []
        grouped = None
        while True:
            boundary = self.at
            while self.peek().kind == 'newline':
                self.pop()
            marker = self.peek()
            if marker.scol != opening.scol or marker.text not in ('except', 'else', 'finally'):
                self.at = boundary
                break
            self.pop()
            if marker.text == 'except':
                starred = self.at_op('*')
                if starred:
                    self.pop()
                if grouped is not None and grouped != starred:
                    _syntax("cannot have both 'except' and 'except*' on the same 'try'")
                grouped = starred
                if otherwise or finalbody or (handlers and handlers[-1].type is None):
                    _syntax('invalid syntax')
                exception_type = None
                name = None
                if not self.at_op(':'):
                    exception_type, unused, end = self.parse_expr(False)
                    if self.at_name('as'):
                        self.pop()
                        bound = self.pop()
                        if bound.kind != 'name':
                            _syntax('invalid syntax')
                        name = bound.text
                elif starred:
                    _syntax('expected one or more exception types')
                handler_body = self.suite(marker)
                last = handler_body[-1]
                handlers.append(self.positioned(ExceptHandler, (marker.srow, marker.scol),
                                (last.end_lineno, last.end_col_offset), type=exception_type,
                                name=name, body=handler_body))
            elif marker.text == 'else':
                if not handlers or otherwise or finalbody:
                    _syntax('invalid syntax')
                otherwise = self.suite(marker)
            else:
                if finalbody:
                    _syntax('invalid syntax')
                finalbody = self.suite(marker)
        if not handlers and not finalbody:
            _syntax("expected 'except' or 'finally' block")
        last = (finalbody or otherwise or (handlers[-1].body if handlers else body))[-1]
        return self.positioned(TryStar if grouped else Try, (opening.srow, opening.scol),
                               (last.end_lineno, last.end_col_offset), body=body,
                               handlers=handlers, orelse=otherwise, finalbody=finalbody)

# The same short text is parsed again and again (a traceback shows one source
# line for every frame of a recursion, twice over): the tree of each recent text
# is kept, and every caller is handed a fresh copy, so no caller sees another's
# changes. Faults are not kept, and long texts are parsed afresh.
_ParseUncached = parse
_TreeCache = {}

def _copy_tree(node):
    kind = node.__class__
    copy = kind.__new__(kind)
    held = node.__dict__.copy()
    for name in kind._fields:
        value = held.get(name)
        if type(value) is list:
            held[name] = [_copy_tree(item) if isinstance(item, AST) else item for item in value]
        elif isinstance(value, AST):
            held[name] = _copy_tree(value)
    copy.__dict__.update(held)
    return copy

def parse(source, filename='<unknown>', mode='exec', *, type_comments=False,
          feature_version=None, optimize=-1, fold_debug=False):
    if type(source) is not str or len(source) > 1024:
        return _ParseUncached(source, filename, mode, type_comments=type_comments,
                              feature_version=feature_version, optimize=optimize, fold_debug=fold_debug)
    key = (source, mode, type_comments, feature_version, optimize, fold_debug)
    kept = _TreeCache.get(key)
    if kept is None:
        tree = _ParseUncached(source, filename, mode, type_comments=type_comments,
                              feature_version=feature_version, optimize=optimize, fold_debug=fold_debug)
        if len(_TreeCache) >= 64:
            _TreeCache.clear()
        _TreeCache[key] = _copy_tree(tree)
        return tree
    return _copy_tree(kept)
