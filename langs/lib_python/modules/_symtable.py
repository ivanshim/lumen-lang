"""Scope analysis behind the symtable module.

The compiler's symbol tables are rebuilt here from the syntax tree: a first
pass records how every name is used or bound in each block, a second pass
resolves each name to a local, cell, free, or global scope.  The results are
exposed in the shape the symtable module reads.
"""

import ast
import os

USE = 16
DEF_GLOBAL = 1
DEF_NONLOCAL = 8
DEF_LOCAL = 2
DEF_PARAM = 4
DEF_TYPE_PARAM = 1024
DEF_FREE = 32
DEF_FREE_CLASS = 64
DEF_IMPORT = 128
DEF_ANNOT = 256
DEF_COMP_ITER = 512
DEF_COMP_CELL = 2048
DEF_BOUND = DEF_LOCAL | DEF_PARAM | DEF_IMPORT

SCOPE_OFF = 12
SCOPE_MASK = 15

LOCAL = 1
GLOBAL_EXPLICIT = 2
GLOBAL_IMPLICIT = 3
FREE = 4
CELL = 5

TYPE_FUNCTION = 0
TYPE_CLASS = 1
TYPE_MODULE = 2
TYPE_ANNOTATION = 3
TYPE_TYPE_ALIAS = 4
TYPE_TYPE_PARAMETERS = 5
TYPE_TYPE_VARIABLE = 6

_FUNCTION_LIKE = (TYPE_FUNCTION, TYPE_ANNOTATION, TYPE_TYPE_ALIAS,
                  TYPE_TYPE_PARAMETERS, TYPE_TYPE_VARIABLE)

_CLASS_PRIVATE = ('__class__', '__classdict__', '__conditional_annotations__')

_COMPREHENSION_NAMES = {
    'ListComp': 'list comprehension',
    'SetComp': 'set comprehension',
    'DictComp': 'dict comprehension',
    'GeneratorExp': 'generator expression',
}


class _Entry:
    """One block of the program: a module, class, function or similar scope."""

    def __init__(self, ident, name, kind, lineno):
        self.id = ident
        self.name = name
        self.type = kind
        self.lineno = lineno
        self.nested = False
        self.children = []
        self.symbols = {}
        self.varnames = []
        self.directives = {}
        self.comprehension = None
        self.generator = False
        self.coroutine = False
        self.can_see_class_scope = False
        self.comp_inlined = False
        self.comp_iter_target = False
        self.comp_iter_expr = 0
        self.free = False
        self.child_free = False
        self.annotation_block = None
        self.in_conditional = 0
        self.mangled_names = None
        self.scope_info = None

    def __repr__(self):
        return '<symtable entry %s(%d), line %d>' % (self.name, self.id, self.lineno)

    @property
    def identifiers(self):
        return list(self.symbols)


def _syntax_error(message, filename, lineno, col, end_lineno, end_col):
    error = SyntaxError(message)
    error.filename = filename
    error.lineno = lineno
    error.offset = col + 1
    error.end_lineno = end_lineno
    error.end_offset = end_col + 1
    return error


class _Location:
    def __init__(self, node):
        self.lineno = getattr(node, 'lineno', 1)
        self.col = getattr(node, 'col_offset', 0)
        self.end_lineno = getattr(node, 'end_lineno', None) or self.lineno
        self.end_col = getattr(node, 'end_col_offset', None)
        if self.end_col is None:
            self.end_col = self.col


def _mangle(private, name):
    if private is None or not name.startswith('__') or name.endswith('__') or '.' in name:
        return name
    stripped = private.lstrip('_')
    if not stripped:
        return name
    return '_' + stripped + name


_FUTURE_FEATURES = ('nested_scopes', 'generators', 'division', 'absolute_import', 'with_statement',
                    'print_function', 'unicode_literals', 'barry_as_FLUFL', 'generator_stop',
                    'annotations')


def _future_annotations(tree, filename):
    """Check the __future__ statements and report whether annotations are deferred."""
    body = getattr(tree, 'body', None)
    if not isinstance(body, list):
        return False
    deferred = False
    done = False
    for position, stmt in enumerate(body):
        if (position == 0 and isinstance(stmt, ast.Expr) and isinstance(stmt.value, ast.Constant)
                and isinstance(stmt.value.value, str)):
            continue
        if not (isinstance(stmt, ast.ImportFrom) and stmt.module == '__future__' and not stmt.level):
            done = True
            continue
        loc = _Location(stmt)
        where = (filename, loc.lineno, loc.col, loc.end_lineno, loc.end_col)
        if done:
            raise _syntax_error('from __future__ imports must occur at the beginning of the file',
                                *where)
        for alias in stmt.names:
            if alias.name not in _FUTURE_FEATURES:
                raise _syntax_error('future feature %s is not defined' % alias.name, *where)
            if alias.name == 'annotations':
                deferred = True
    return deferred


class _Builder:
    def __init__(self, filename, future_annotations):
        self.filename = filename
        self.future_annotations = future_annotations
        self.stack = []
        self.cur = None
        self.top = None
        self.global_symbols = None
        self.private = None
        self.counter = 0

    # -- errors ----------------------------------------------------------

    def error(self, message, where):
        loc = where if isinstance(where, _Location) else _Location(where)
        raise _syntax_error(message, self.filename, loc.lineno, loc.col, loc.end_lineno, loc.end_col)

    # -- blocks ----------------------------------------------------------

    def enter_block(self, name, kind, node, lineno=None):
        self.counter += 1
        ste = _Entry(self.counter, name, kind, node.lineno if lineno is None else lineno)
        prev = self.cur
        if prev is not None and (prev.nested or prev.type in _FUNCTION_LIKE):
            ste.nested = True
        self.stack.append(ste)
        self.cur = ste
        if kind in (TYPE_ANNOTATION, TYPE_TYPE_ALIAS, TYPE_TYPE_VARIABLE):
            self.add_def('.format', DEF_PARAM, node)
            self.add_def('.format', USE, node)
        if kind == TYPE_MODULE:
            self.global_symbols = ste.symbols
        if prev is not None:
            prev.children.append(ste)
        return ste

    def enter_existing(self, ste):
        self.stack.append(ste)
        self.cur = ste

    def exit_block(self):
        self.stack.pop()
        self.cur = self.stack[-1] if self.stack else None

    def enter_type_params(self, name, node, defaults, kwdefaults, is_class_def):
        enclosing = self.cur.type
        ste = self.enter_block(name, TYPE_TYPE_PARAMETERS, node)
        if enclosing == TYPE_CLASS:
            ste.can_see_class_scope = True
            self.add_def('__classdict__', USE, node)
        if is_class_def:
            self.add_def('.type_params', DEF_LOCAL, node)
            self.add_def('.type_params', USE, node)
            self.private = name
            ste.mangled_names = set()
            self.add_def('.generic_base', DEF_LOCAL, node)
            self.add_def('.generic_base', USE, node)
        if defaults:
            self.add_def('.defaults', DEF_PARAM, node)
        if kwdefaults:
            self.add_def('.kwdefaults', DEF_PARAM, node)

    # -- symbols ---------------------------------------------------------

    def mangled(self, ste, name):
        if ste.mangled_names is not None and name not in ste.mangled_names:
            return name
        return _mangle(self.private, name)

    def lookup(self, name):
        return self.cur.symbols.get(self.mangled(self.cur, name), 0)

    def add_def(self, name, flag, where, ste=None):
        ste = ste or self.cur
        name = self.mangled(ste, name)
        symbols = ste.symbols
        if name in symbols:
            val = symbols[name]
            if flag & DEF_PARAM and val & DEF_PARAM:
                self.error("duplicate argument '%s' in function definition" % name, where)
            if flag & DEF_TYPE_PARAM and val & DEF_TYPE_PARAM:
                self.error("duplicate type parameter '%s'" % name, where)
            val |= flag
        else:
            val = flag
        if ste.comp_iter_target:
            if val & (DEF_GLOBAL | DEF_NONLOCAL):
                self.error("comprehension inner loop cannot rebind assignment expression target '%s'"
                           % name, where)
            val |= DEF_COMP_ITER
        symbols[name] = val
        if flag & DEF_PARAM:
            ste.varnames.append(name)
        elif flag & DEF_GLOBAL:
            val = flag | self.global_symbols.get(name, 0)
            self.global_symbols[name] = val

    def record_directive(self, name, node):
        self.cur.directives.setdefault(self.mangled(self.cur, name), _Location(node))

    # -- traversal helpers -----------------------------------------------

    def visit(self, node):
        if node is None:
            return
        method = getattr(self, 'visit_' + type(node).__name__, None)
        if method is None:
            for child in ast.iter_child_nodes(node):
                self.visit(child)
        else:
            method(node)

    def visit_all(self, nodes):
        for node in nodes:
            self.visit(node)

    def conditional(self, *groups):
        self.cur.in_conditional += 1
        for group in groups:
            self.visit_all(group)
        self.cur.in_conditional -= 1

    def reject_in_special_block(self, what, node):
        kind = self.cur.type
        if kind == TYPE_ANNOTATION:
            self.error('%s cannot be used within an annotation' % what, node)
        if kind == TYPE_TYPE_VARIABLE:
            self.error('%s cannot be used within %s' % (what, self.cur.scope_info), node)
        if kind == TYPE_TYPE_ALIAS:
            self.error('%s cannot be used within a type alias' % what, node)
        if kind == TYPE_TYPE_PARAMETERS:
            self.error('%s cannot be used within the definition of a generic' % what, node)

    # -- module ----------------------------------------------------------

    def build(self, tree):
        top = self.enter_block('top', TYPE_MODULE, tree, 0)
        self.top = top
        if isinstance(tree, ast.Expression):
            self.visit(tree.body)
        else:
            self.visit_all(tree.body)
        self.exit_block()
        return top

    # -- statements ------------------------------------------------------

    def visit_FunctionDef(self, node):
        self.add_def(node.name, DEF_LOCAL, node)
        args = node.args
        self.visit_all(args.defaults)
        self.visit_all(args.kw_defaults)
        self.visit_all(node.decorator_list)
        generic = bool(node.type_params)
        if generic:
            self.enter_type_params(node.name, node, True,
                                   any(d is not None for d in args.kw_defaults), False)
            self.visit_all(node.type_params)
        self.visit_annotations(node, args, node.returns)
        self.enter_block(node.name, TYPE_FUNCTION, node)
        if isinstance(node, ast.AsyncFunctionDef):
            self.cur.coroutine = True
        self.visit_arguments(args)
        self.visit_all(node.body)
        self.exit_block()
        if generic:
            self.exit_block()

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_annotations(self, node, args, returns):
        if self.future_annotations:
            return
        in_class = self.cur.can_see_class_scope or self.cur.type == TYPE_CLASS
        ste = self.enter_block('__annotate__', TYPE_ANNOTATION, node)
        if in_class:
            ste.can_see_class_scope = True
            self.add_def('__classdict__', USE, node)
        for group in (args.posonlyargs, args.args):
            for arg in group:
                self.visit(arg.annotation)
        if args.vararg is not None:
            self.visit(args.vararg.annotation)
        if args.kwarg is not None:
            self.visit(args.kwarg.annotation)
        for arg in args.kwonlyargs:
            self.visit(arg.annotation)
        self.visit(returns)
        self.exit_block()

    def visit_arguments(self, args):
        for group in (args.posonlyargs, args.args, args.kwonlyargs):
            for arg in group:
                self.add_def(arg.arg, DEF_PARAM, arg)
        if args.vararg is not None:
            self.add_def(args.vararg.arg, DEF_PARAM, args.vararg)
        if args.kwarg is not None:
            self.add_def(args.kwarg.arg, DEF_PARAM, args.kwarg)

    def visit_ClassDef(self, node):
        self.add_def(node.name, DEF_LOCAL, node)
        self.visit_all(node.decorator_list)
        generic = bool(node.type_params)
        saved_private = self.private
        if generic:
            self.enter_type_params(node.name, node, False, False, True)
            self.visit_all(node.type_params)
        self.visit_all(node.bases)
        self.visit_all(node.keywords)
        previous = self.private
        self.private = node.name
        self.enter_block(node.name, TYPE_CLASS, node)
        if generic:
            self.add_def('__type_params__', DEF_LOCAL, node)
            self.add_def('.type_params', USE, node)
        self.visit_all(node.body)
        self.exit_block()
        self.private = previous
        if generic:
            self.exit_block()
            self.private = saved_private

    def visit_Return(self, node):
        self.visit(node.value)

    def visit_Delete(self, node):
        self.visit_all(node.targets)

    def visit_Assign(self, node):
        self.visit_all(node.targets)
        self.visit(node.value)

    def visit_AugAssign(self, node):
        self.visit(node.target)
        self.visit(node.value)

    def visit_AnnAssign(self, node):
        target = node.target
        if isinstance(target, ast.Name):
            current = self.lookup(target.id)
            if (current & (DEF_GLOBAL | DEF_NONLOCAL) and self.cur.symbols is not self.global_symbols
                    and node.simple):
                self.error("annotated name '%s' can't be %s"
                           % (target.id, 'global' if current & DEF_GLOBAL else 'nonlocal'), node)
            if node.simple:
                self.add_def(target.id, DEF_ANNOT | DEF_LOCAL, target)
                if node.value is not None:
                    self.add_def(target.id, DEF_LOCAL, target)
            elif node.value is not None:
                self.add_def(target.id, DEF_LOCAL, target)
        else:
            self.visit(target)
        self.visit_annotation(node.annotation)
        self.visit(node.value)

    def visit_annotation(self, annotation):
        parent = self.cur
        kind = parent.type
        if kind in (TYPE_MODULE, TYPE_CLASS) and (kind == TYPE_MODULE or parent.in_conditional):
            self.add_def('__conditional_annotations__', USE, annotation)
        if self.future_annotations:
            return
        if parent.annotation_block is None:
            ste = self.enter_block('__annotate__', TYPE_ANNOTATION, annotation)
            parent.annotation_block = ste
            if kind == TYPE_CLASS:
                ste.can_see_class_scope = True
                self.add_def('__classdict__', USE, annotation)
        else:
            self.enter_existing(parent.annotation_block)
        # Annotations of local variables are never evaluated.
        if kind in (TYPE_MODULE, TYPE_CLASS):
            self.visit(annotation)
        self.exit_block()

    def visit_For(self, node):
        self.visit(node.target)
        self.visit(node.iter)
        self.conditional(node.body, node.orelse)

    visit_AsyncFor = visit_For

    def visit_While(self, node):
        self.visit(node.test)
        self.conditional(node.body, node.orelse)

    def visit_If(self, node):
        self.visit(node.test)
        self.conditional(node.body, node.orelse)

    def visit_With(self, node):
        self.cur.in_conditional += 1
        for item in node.items:
            self.visit(item.context_expr)
            self.visit(item.optional_vars)
        self.visit_all(node.body)
        self.cur.in_conditional -= 1

    visit_AsyncWith = visit_With

    def visit_Match(self, node):
        self.visit(node.subject)
        self.cur.in_conditional += 1
        self.visit_all(node.cases)
        self.cur.in_conditional -= 1

    def visit_match_case(self, node):
        self.visit(node.pattern)
        self.visit(node.guard)
        self.visit_all(node.body)

    def visit_Raise(self, node):
        self.visit(node.exc)
        self.visit(node.cause)

    def visit_Try(self, node):
        self.cur.in_conditional += 1
        self.visit_all(node.body)
        self.visit_all(node.handlers)
        self.visit_all(node.orelse)
        self.visit_all(node.finalbody)
        self.cur.in_conditional -= 1

    visit_TryStar = visit_Try

    def visit_ExceptHandler(self, node):
        self.visit(node.type)
        if node.name:
            self.add_def(node.name, DEF_LOCAL, node)
        self.visit_all(node.body)

    def visit_Assert(self, node):
        self.visit(node.test)
        self.visit(node.msg)

    def visit_Import(self, node):
        self.visit_all(node.names)

    def visit_ImportFrom(self, node):
        self.visit_all(node.names)

    def visit_alias(self, node):
        name = node.asname or node.name
        if node.asname is None:
            name = node.name.split('.')[0]
        if node.name != '*':
            self.add_def(name, DEF_IMPORT, node)
        elif self.cur.type != TYPE_MODULE:
            self.error('import * only allowed at module level', node)

    def declare(self, node, flag, word):
        for name in node.names:
            current = self.lookup(name)
            if current & (DEF_PARAM | DEF_LOCAL | USE | DEF_ANNOT):
                if current & DEF_PARAM:
                    message = "name '%s' is parameter and %s" % (name, word)
                elif current & USE:
                    message = "name '%s' is used prior to %s declaration" % (name, word)
                elif current & DEF_ANNOT:
                    message = "annotated name '%s' can't be %s" % (name, word)
                else:
                    message = "name '%s' is assigned to before %s declaration" % (name, word)
                self.error(message, node)
            self.add_def(name, flag, node)
            self.record_directive(name, node)

    def visit_Global(self, node):
        self.declare(node, DEF_GLOBAL, 'global')

    def visit_Nonlocal(self, node):
        self.declare(node, DEF_NONLOCAL, 'nonlocal')

    def visit_Expr(self, node):
        self.visit(node.value)

    def visit_TypeAlias(self, node):
        self.visit(node.name)
        name = node.name.id
        in_class = self.cur.type == TYPE_CLASS
        generic = bool(node.type_params)
        if generic:
            self.enter_type_params(name, node, False, False, False)
            self.visit_all(node.type_params)
        ste = self.enter_block(name, TYPE_TYPE_ALIAS, node)
        ste.can_see_class_scope = in_class
        if in_class:
            self.add_def('__classdict__', USE, node.name)
        self.visit(node.value)
        self.exit_block()
        if generic:
            self.exit_block()

    # -- patterns --------------------------------------------------------

    def visit_MatchAs(self, node):
        self.visit(node.pattern)
        if node.name:
            self.add_def(node.name, DEF_LOCAL, node)

    def visit_MatchStar(self, node):
        if node.name:
            self.add_def(node.name, DEF_LOCAL, node)

    def visit_MatchMapping(self, node):
        self.visit_all(node.keys)
        self.visit_all(node.patterns)
        if node.rest:
            self.add_def(node.rest, DEF_LOCAL, node)

    def visit_MatchClass(self, node):
        self.visit(node.cls)
        self.visit_all(node.patterns)
        self.visit_all(node.kwd_patterns)

    # -- expressions -----------------------------------------------------

    def visit_Name(self, node):
        load = isinstance(node.ctx, ast.Load)
        self.add_def(node.id, USE if load else DEF_LOCAL, node)
        if load and self.cur.type in _FUNCTION_LIKE and node.id == 'super':
            self.add_def('__class__', USE, node)

    def visit_NamedExpr(self, node):
        self.reject_in_special_block('named expression', node)
        if self.cur.comp_iter_expr > 0:
            self.error('assignment expression cannot be used in a comprehension iterable '
                       'expression', node)
        if self.cur.comprehension:
            self.extend_namedexpr_scope(node.target)
        self.visit(node.value)
        self.visit(node.target)

    def extend_namedexpr_scope(self, target):
        name = target.id
        for ste in reversed(self.stack):
            if ste.comprehension:
                flags = ste.symbols.get(self.mangled(ste, name), 0)
                if flags & DEF_COMP_ITER and flags & DEF_LOCAL:
                    self.error("assignment expression cannot rebind comprehension iteration "
                               "variable '%s'" % name, target)
                continue
            if ste.type == TYPE_FUNCTION:
                flags = ste.symbols.get(self.mangled(ste, name), 0)
                self.add_def(name, DEF_GLOBAL if flags & DEF_GLOBAL else DEF_NONLOCAL, target)
                self.record_directive(name, target)
                self.add_def(name, DEF_LOCAL, target, ste)
                return
            if ste.type == TYPE_MODULE:
                self.add_def(name, DEF_GLOBAL, target)
                self.record_directive(name, target)
                self.add_def(name, DEF_GLOBAL, target, ste)
                return
            if ste.type == TYPE_CLASS:
                self.error('assignment expression within a comprehension cannot be used in a '
                           'class body', target)
            if ste.type == TYPE_TYPE_ALIAS:
                self.error('assignment expression within a comprehension cannot be used in a '
                           'type alias', target)
            if ste.type == TYPE_TYPE_PARAMETERS:
                self.error('assignment expression within a comprehension cannot be used within '
                           'the definition of a generic', target)
            if ste.type == TYPE_TYPE_VARIABLE:
                self.error('assignment expression within a comprehension cannot be used in %s'
                           % ste.scope_info, target)
        raise SystemError('no enclosing block for assignment expression')

    def visit_Lambda(self, node):
        self.visit_all(node.args.defaults)
        self.visit_all(node.args.kw_defaults)
        self.enter_block('lambda', TYPE_FUNCTION, node)
        self.visit_arguments(node.args)
        self.visit(node.body)
        self.exit_block()

    def visit_Dict(self, node):
        self.visit_all(node.keys)
        self.visit_all(node.values)

    def visit_Yield(self, node):
        self.reject_in_special_block('yield expression', node)
        self.visit(node.value)
        self.cur.generator = True
        self.reject_in_comprehension(node)

    def visit_YieldFrom(self, node):
        self.reject_in_special_block('yield expression', node)
        self.visit(node.value)
        self.cur.generator = True
        self.reject_in_comprehension(node)

    def visit_Await(self, node):
        self.reject_in_special_block('await expression', node)
        if self.cur.type not in _FUNCTION_LIKE:
            self.error("'await' outside function", node)
        if not (self.cur.type == TYPE_FUNCTION and self.cur.coroutine) and not self.cur.comprehension:
            self.error("'await' outside async function", node)
        self.visit(node.value)
        self.cur.coroutine = True

    def reject_in_comprehension(self, node):
        if self.cur.comprehension:
            self.error("'yield' inside %s" % self.cur.comprehension, node)

    def visit_ListComp(self, node):
        self.comprehension(node, 'listcomp', node.generators, node.elt, None)

    def visit_SetComp(self, node):
        self.comprehension(node, 'setcomp', node.generators, node.elt, None)

    def visit_DictComp(self, node):
        self.comprehension(node, 'dictcomp', node.generators, node.key, node.value)

    def visit_GeneratorExp(self, node):
        self.comprehension(node, 'genexpr', node.generators, node.elt, None)

    def comprehension(self, node, scope_name, generators, elt, value):
        is_generator = isinstance(node, ast.GeneratorExp)
        outermost = generators[0]
        self.cur.comp_iter_expr += 1
        self.visit(outermost.iter)
        self.cur.comp_iter_expr -= 1
        ste = self.enter_block(scope_name, TYPE_FUNCTION, node)
        ste.comprehension = _COMPREHENSION_NAMES[type(node).__name__]
        if outermost.is_async:
            ste.coroutine = True
        self.add_def('.0', DEF_PARAM, node)
        ste.comp_iter_target = True
        self.visit(outermost.target)
        ste.comp_iter_target = False
        self.visit_all(outermost.ifs)
        for generator in generators[1:]:
            self.visit_comprehension(generator)
        if value is not None:
            self.visit(value)
        self.visit(elt)
        ste.generator = is_generator
        is_async = ste.coroutine and not is_generator
        self.exit_block()
        if is_async:
            enclosing = self.cur
            if not (enclosing.type == TYPE_FUNCTION and enclosing.coroutine) and not enclosing.comprehension:
                self.error('asynchronous comprehension outside of an asynchronous function', node)
            enclosing.coroutine = True

    def visit_comprehension(self, node):
        self.cur.comp_iter_target = True
        self.visit(node.target)
        self.cur.comp_iter_target = False
        self.cur.comp_iter_expr += 1
        self.visit(node.iter)
        self.cur.comp_iter_expr -= 1
        self.visit_all(node.ifs)
        if node.is_async:
            self.cur.coroutine = True

    # -- type parameters ---------------------------------------------------

    def visit_TypeVar(self, node):
        self.declare_type_parameter(node)
        info = 'a TypeVar constraint' if isinstance(node.bound, ast.Tuple) else 'a TypeVar bound'
        self.type_variable_block(node, node.bound, info)
        self.type_variable_block(node, node.default_value, 'a TypeVar default')

    def visit_ParamSpec(self, node):
        self.declare_type_parameter(node)
        self.type_variable_block(node, node.default_value, 'a ParamSpec default')

    def visit_TypeVarTuple(self, node):
        self.declare_type_parameter(node)
        self.type_variable_block(node, node.default_value, 'a TypeVarTuple default')

    def declare_type_parameter(self, node):
        if self.cur.mangled_names is not None:
            self.cur.mangled_names.add(node.name)
        self.add_def(node.name, DEF_TYPE_PARAM | DEF_LOCAL, node)

    def type_variable_block(self, node, expr, info):
        if expr is None:
            return
        in_class = self.cur.can_see_class_scope
        ste = self.enter_block(node.name, TYPE_TYPE_VARIABLE, node)
        ste.scope_info = info
        ste.can_see_class_scope = in_class
        if in_class:
            self.add_def('__classdict__', USE, expr)
        self.visit(expr)
        self.exit_block()


# -- analysis --------------------------------------------------------------

class _Analyzer:
    def __init__(self, filename):
        self.filename = filename

    def error(self, message, ste, name):
        loc = ste.directives.get(name)
        if loc is None:
            raise _syntax_error(message, self.filename, ste.lineno, 0, ste.lineno, 0)
        raise _syntax_error(message, self.filename, loc.lineno, loc.col, loc.end_lineno, loc.end_col)

    def analyze_name(self, ste, scopes, name, flags, bound, local, free, global_, type_params,
                     class_entry):
        if flags & DEF_GLOBAL:
            if flags & DEF_NONLOCAL:
                self.error("name '%s' is nonlocal and global" % name, ste, name)
            scopes[name] = GLOBAL_EXPLICIT
            global_.add(name)
            if bound is not None:
                bound.discard(name)
            return
        if flags & DEF_NONLOCAL:
            if bound is None:
                self.error('nonlocal declaration not allowed at module level', ste, name)
            if name not in bound:
                self.error("no binding for nonlocal '%s' found" % name, ste, name)
            scopes[name] = FREE
            ste.free = True
            free.add(name)
            return
        if flags & DEF_BOUND:
            scopes[name] = LOCAL
            local.add(name)
            global_.discard(name)
            if flags & DEF_TYPE_PARAM:
                type_params.add(name)
            else:
                type_params.discard(name)
            return
        if class_entry is not None and ste.can_see_class_scope:
            class_flags = class_entry.symbols.get(name, 0)
            if class_flags & DEF_GLOBAL:
                scopes[name] = GLOBAL_EXPLICIT
                return
            if class_flags & DEF_BOUND and not class_flags & DEF_NONLOCAL:
                scopes[name] = GLOBAL_IMPLICIT
                return
        if bound is not None and name in bound:
            scopes[name] = FREE
            ste.free = True
            free.add(name)
            return
        if name in global_:
            scopes[name] = GLOBAL_IMPLICIT
            return
        if ste.nested:
            ste.free = True
        scopes[name] = GLOBAL_IMPLICIT

    def analyze_cells(self, scopes, free, inlined_cells):
        for name, scope in scopes.items():
            if scope != LOCAL:
                continue
            if name not in free and name not in inlined_cells:
                continue
            scopes[name] = CELL
            free.discard(name)

    def drop_class_free(self, free):
        free.difference_update(_CLASS_PRIVATE)

    def update_symbols(self, symbols, scopes, bound, free, inlined_cells, class_flag):
        for name in list(symbols):
            flags = symbols[name]
            if name in inlined_cells:
                flags |= DEF_COMP_CELL
            symbols[name] = flags | (scopes[name] << SCOPE_OFF)
        for name in free:
            if name in symbols:
                if class_flag:
                    symbols[name] |= DEF_FREE_CLASS
                continue
            if bound is not None and name not in bound:
                continue
            symbols[name] = FREE << SCOPE_OFF

    def inline_comprehension(self, ste, comp, scopes, comp_free, inlined_cells):
        removed = []
        for name, comp_flags in comp.symbols.items():
            if comp_flags & DEF_PARAM:
                continue
            scope = (comp_flags >> SCOPE_OFF) & SCOPE_MASK
            only_flags = comp_flags & ((1 << SCOPE_OFF) - 1)
            if scope == CELL or only_flags & DEF_COMP_CELL:
                inlined_cells.add(name)
            existing = ste.symbols.get(name)
            hidden = scope == FREE and ste.type == TYPE_CLASS and name in _CLASS_PRIVATE
            if hidden:
                comp_free.discard(name)
                removed.append(name)
            if existing is None:
                if hidden:
                    scope = GLOBAL_IMPLICIT
                    only_flags &= ~DEF_FREE
                ste.symbols[name] = only_flags
                scopes[name] = scope
            elif (existing & DEF_BOUND and ste.type != TYPE_CLASS
                  and not self.free_in_a_child(comp, name)):
                comp_free.discard(name)
        for name in removed:
            del comp.symbols[name]

    def free_in_a_child(self, ste, name):
        for child in ste.children:
            if (child.symbols.get(name, 0) >> SCOPE_OFF) & SCOPE_MASK == FREE:
                return True
        return False

    def analyze_block(self, ste, bound, free, global_, type_params, class_entry):
        local = set()
        scopes = {}
        new_global = set()
        new_free = set()
        new_bound = set()
        inlined_cells = set()
        is_class = ste.type == TYPE_CLASS
        if is_class:
            new_global |= global_
            if bound is not None:
                new_bound |= bound
        for name, flags in list(ste.symbols.items()):
            self.analyze_name(ste, scopes, name, flags, bound, local, free, global_,
                              type_params, class_entry)
        if not is_class:
            if ste.type in _FUNCTION_LIKE:
                new_bound |= local
            if bound is not None:
                new_bound |= bound
            new_global |= global_
        else:
            new_bound.add('__class__')
            new_bound.add('__classdict__')
            new_bound.add('__conditional_annotations__')
        if is_class:
            child_class = ste
        elif ste.can_see_class_scope:
            child_class = class_entry
        else:
            child_class = None
        for child in ste.children:
            inline = (child.comprehension is not None and not child.generator
                      and not ste.can_see_class_scope)
            child_free = self.analyze_child(child, new_bound, new_free, new_global, type_params,
                                            child_class)
            if inline:
                self.inline_comprehension(ste, child, scopes, child_free, inlined_cells)
                child.comp_inlined = True
            new_free |= child_free
            if child.free or child.child_free:
                ste.child_free = True
        spliced = []
        for child in ste.children:
            if child.comp_inlined:
                spliced.extend(child.children)
            else:
                spliced.append(child)
        ste.children = spliced
        if ste.type in _FUNCTION_LIKE:
            self.analyze_cells(scopes, new_free, inlined_cells)
        elif is_class:
            self.drop_class_free(new_free)
        self.update_symbols(ste.symbols, scopes, bound, new_free, inlined_cells,
                            is_class or ste.can_see_class_scope)
        free |= new_free

    def analyze_child(self, child, bound, free, global_, type_params, class_entry):
        child_bound = set(bound)
        child_free = set(free)
        child_global = set(global_)
        child_type_params = set(type_params)
        self.analyze_block(child, child_bound, child_free, child_global, child_type_params,
                           class_entry)
        return child_free


def _parse(source, filename, mode):
    try:
        return ast.parse(source, filename, mode)
    except SyntaxError as error:
        if error.lineno is None:
            # The tree reader gives no position; the compiler's own report does.
            compile(source, filename, mode, dont_inherit=True)
        raise


def symtable(source, filename, compile_type):
    filename = os.fspath(filename)
    if isinstance(filename, bytes):
        filename = os.fsdecode(filename)
    if not isinstance(compile_type, str):
        raise TypeError('symtable() argument 3 must be str, not %s' % type(compile_type).__name__)
    if compile_type not in ('exec', 'eval', 'single'):
        raise ValueError("symtable() arg 3 must be 'exec' or 'eval' or 'single'")
    tree = _parse(source, filename, compile_type)
    builder = _Builder(filename, _future_annotations(tree, filename))
    top = builder.build(tree)
    _Analyzer(filename).analyze_block(top, None, set(), set(), set(), None)
    return top
