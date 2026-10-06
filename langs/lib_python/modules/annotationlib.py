# The formats an __annotate__ function is asked for, and the one reader of
# annotations that works without one. This runtime records the names a class
# or module annotates but does not evaluate the annotations themselves, so
# VALUE and resolved FORWARDREF annotations can be served; formats requiring
# the original annotation expression are refused.

class Format:
    VALUE = 1
    VALUE_WITH_FAKE_GLOBALS = 2
    FORWARDREF = 3
    STRING = 4

_FORMAT_NAMES = {1: 'VALUE', 2: 'VALUE_WITH_FAKE_GLOBALS', 3: 'FORWARDREF', 4: 'STRING'}

def _check_format(format):
    if format not in _FORMAT_NAMES:
        raise 'ValueError: ' + str(format) + ' is not a valid Format'
    if format == Format.VALUE_WITH_FAKE_GLOBALS:
        raise ValueError("The VALUE_WITH_FAKE_GLOBALS format is for internal use only")

def get_annotate_from_class_namespace(obj):
    # A class body here never leaves an __annotate__ behind.
    try:
        return obj['__annotate__']
    except Exception:
        try:
            rows = obj['\0annotate']
        except KeyError:
            return None
        def annotate(format):
            _check_format(format)
            result = {}
            for index in range(0, len(rows), 2):
                result[rows[index]] = rows[index + 1]()
            return result
        return annotate

def call_annotate_function(annotate, format, owner=None):
    _check_format(format)
    # An annotate function may give the format itself: the ones written
    # down here do for the text format, and dataclasses does for both.
    try:
        return annotate(format)
    except NotImplementedError:
        pass
    if format == Format.STRING:
        # Without the written form, the values stand for their own text,
        # names not yet defined among them as the text they were read by.
        return {key: _string_of(value) for key, value in _evaluate_symbolic(annotate, owner, Format.FORWARDREF).items()}
    if format == Format.FORWARDREF:
        return _evaluate_symbolic(annotate, owner, format)
    raise ValueError(f"Invalid format: {format!r}")

def _string_of(value):
    # A value as the text format writes it, a reference to be resolved
    # later as the text it stands for, wherever in the value it is.
    if isinstance(value, str):
        return value
    if isinstance(value, ForwardRef):
        text = value.__forward_arg__
        names = value.__extra_names__ or {}
        for unique in sorted(names, key=len, reverse=True):
            text = text.replace(unique, _string_of(names[unique]))
        return text
    if hasattr(value, '__origin__') and hasattr(value, '__args__') and _holds_reference(value):
        return _string_of(value.__origin__) + '[' + ', '.join(_string_of(each) for each in value.__args__) + ']'
    return type_repr(value)

def _holds_reference(value):
    if isinstance(value, ForwardRef):
        return True
    args = getattr(value, '__args__', None)
    return isinstance(args, tuple) and any(_holds_reference(each) for each in args)

def _home_of(annotate, owner):
    # The module whose names the annotations are read among.
    for thing in (annotate, owner):
        name = getattr(thing, '__module__', None)
        if isinstance(name, str) and name in sys.modules:
            return sys.modules[name]
    return None

def _evaluate_symbolic(annotate, owner, format):
    """Read the annotations in their own scope. A name that is nowhere
    defined stands, for as long as they are read, as a reference to be
    resolved later; any other error is the caller's."""
    try:
        return annotate(Format.VALUE)
    except NameError as error:
        first = error
    home = _home_of(annotate, owner)
    if home is None or getattr(first, 'name', None) is None:
        raise first
    names = _StringifierDict(
        {}, globals=getattr(annotate, '__globals__', None), owner=owner,
        is_class=isinstance(owner, type), format=format)
    stood = []
    try:
        while True:
            names.next_id = 1
            try:
                result = annotate(Format.VALUE)
                break
            except NameError as error:
                name = getattr(error, 'name', None)
                if name is None or name in stood or hasattr(home, name):
                    raise
                setattr(home, name, names[name])
                stood.append(name)
    finally:
        for name in stood:
            if name in vars(home) and isinstance(vars(home)[name], _Stringifier):
                delattr(home, name)
    names.transmogrify(None)
    return result

def call_evaluate_function(evaluate, format, owner=None):
    _check_format(format)
    return evaluate(format)

def get_annotations(obj, *, globals=None, locals=None, eval_str=False, format=1):
    _check_format(format)
    annotate = getattr(obj, '__annotate__', None)
    if annotate is not None:
        result = call_annotate_function(annotate, format, owner=obj)
        if result is None:
            return {}
        return dict(result)
    stored = getattr(obj, '__annotations__', None)
    if stored is None:
        return {}
    if eval_str:
        raise 'NotImplementedError: annotationlib cannot evaluate string annotations here'
    if format == Format.STRING:
        return {key: value if isinstance(value, str) else type_repr(value) for key, value in stored.items()}
    return dict(stored)

# Runtime adapter derived from CPython v3.14.8 / 8e6e75d9102e, Lib/annotationlib.py; PSF License.
# ForwardRef uses conditionals for formats; symbolic AST transformation is unavailable.
import ast
import builtins
import keyword
import sys
import types
_NAME_ERROR_MSG = "name '{name:.200}' is not defined"
_SLOTS = (
    "__forward_is_argument__",
    "__forward_is_class__",
    "__forward_module__",
    "__weakref__",
    "__arg__",
    "__globals__",
    "__extra_names__",
    "__code__",
    "__ast_node__",
    "__cell__",
    "__owner__",
    "__stringifier_dict__",
    "__resolved_str_cache__",
)


class ForwardRef:
    """Wrapper that holds a forward reference.

    Constructor arguments:
    * arg: a string representing the code to be evaluated.
    * module: the module where the forward reference was created.
      Must be a string, not a module object.
    * owner: The owning object (module, class, or function).
    * is_argument: Does nothing, retained for compatibility.
    * is_class: True if the forward reference was created in class scope.

    """

    __slots__ = _SLOTS

    def __init__(
        self,
        arg,
        *,
        module=None,
        owner=None,
        is_argument=True,
        is_class=False,
    ):
        if not isinstance(arg, str):
            raise TypeError(f"Forward reference must be a string -- got {arg!r}")

        self.__arg__ = arg
        self.__forward_is_argument__ = is_argument
        self.__forward_is_class__ = is_class
        self.__forward_module__ = module
        self.__owner__ = owner
        # These are always set to None here but may be non-None if a ForwardRef
        # is created through __class__ assignment on a _Stringifier object.
        self.__globals__ = None
        # This may be either a cell object (for a ForwardRef referring to a single name)
        # or a dict mapping cell names to cell objects (for a ForwardRef containing references
        # to multiple names).
        self.__cell__ = None
        self.__extra_names__ = None
        # These are initially None but serve as a cache and may be set to a non-None
        # value later.
        self.__code__ = None
        self.__ast_node__ = None
        self.__resolved_str_cache__ = None

    def __init_subclass__(cls, /, *args, **kwds):
        raise TypeError("Cannot subclass ForwardRef")

    def evaluate(
        self,
        *,
        globals=None,
        locals=None,
        type_params=None,
        owner=None,
        format=Format.VALUE,
    ):
        """Evaluate the forward reference and return the value.

        If the forward reference cannot be evaluated, raise an exception.
        """
        if format == Format.STRING:
            return self.__resolved_str__
        elif format == Format.VALUE:
            is_forwardref_format = False
        elif format == Format.FORWARDREF:
            is_forwardref_format = True
        else:
            raise NotImplementedError(format)
        if isinstance(self.__cell__, types.CellType):
            try:
                return self.__cell__.cell_contents
            except ValueError:
                pass
        if owner is None:
            owner = self.__owner__

        if globals is None and self.__forward_module__ is not None:
            globals = getattr(
                sys.modules.get(self.__forward_module__, None), "__dict__", None
            )
        if globals is None:
            globals = self.__globals__
        if globals is None:
            if isinstance(owner, type):
                module_name = getattr(owner, "__module__", None)
                if module_name:
                    module = sys.modules.get(module_name, None)
                    if module:
                        globals = getattr(module, "__dict__", None)
            elif isinstance(owner, types.ModuleType):
                globals = getattr(owner, "__dict__", None)
            elif callable(owner):
                globals = getattr(owner, "__globals__", None)

        # If we pass None to eval() below, the globals of this module are used.
        if globals is None:
            globals = {}

        if type_params is None and owner is not None:
            type_params = getattr(owner, "__type_params__", None)

        if locals is None:
            locals = {}
            if isinstance(owner, type):
                locals.update(vars(owner))
        elif (
            type_params is not None
            or isinstance(self.__cell__, dict)
            or self.__extra_names__
        ):
            # Create a new locals dict if necessary,
            # to avoid mutating the argument.
            locals = dict(locals)

        # "Inject" type parameters into the local namespace
        # (unless they are shadowed by assignments *in* the local namespace),
        # as a way of emulating annotation scopes when calling `eval()`
        if type_params is not None:
            for param in type_params:
                locals.setdefault(param.__name__, param)

        # Similar logic can be used for nonlocals, which should not
        # override locals.
        if isinstance(self.__cell__, dict):
            for cell_name, cell in self.__cell__.items():
                try:
                    cell_value = cell.cell_contents
                except ValueError:
                    pass
                else:
                    locals.setdefault(cell_name, cell_value)

        if self.__extra_names__:
            locals.update(self.__extra_names__)

        arg = self.__forward_arg__
        if arg.isidentifier() and not keyword.iskeyword(arg):
            if arg in locals:
                return locals[arg]
            elif arg in globals:
                return globals[arg]
            elif hasattr(builtins, arg):
                return getattr(builtins, arg)
            elif is_forwardref_format:
                return self
            else:
                raise NameError(_NAME_ERROR_MSG.format(name=arg), name=arg)
        else:
            code = self.__forward_code__
            try:
                return eval(code, globals=globals, locals=locals)
            except Exception:
                if not is_forwardref_format:
                    raise

            # All variables, in scoping order, should be checked before
            # triggering __missing__ to create a _Stringifier.
            new_locals = _StringifierDict(
                {**builtins.__dict__, **globals, **locals},
                globals=globals,
                owner=owner,
                is_class=self.__forward_is_class__,
                format=format,
            )
            try:
                result = eval(code, globals=globals, locals=new_locals)
            except Exception:
                return self
            else:
                new_locals.transmogrify(self.__cell__)
                return result

    @property
    def __forward_arg__(self):
        if self.__arg__ is not None:
            return self.__arg__
        if self.__ast_node__ is not None:
            self.__arg__ = _syn.unparse(self.__ast_node__)
            return self.__arg__
        raise AssertionError(
            "Attempted to access '__forward_arg__' on an uninitialized ForwardRef"
        )

    @property
    def __resolved_str__(self):
        # __forward_arg__ with any names from __extra_names__ replaced
        # with the type_repr of the value they represent
        if self.__resolved_str_cache__ is None:
            resolved_str = self.__forward_arg__
            names = self.__extra_names__

            if names:
                # The names made up for values that have no name stand,
                # in the text, for the way those values are written.
                for unique in sorted(names, key=len, reverse=True):
                    resolved_str = resolved_str.replace(unique, type_repr(names[unique]))

            self.__resolved_str_cache__ = resolved_str

        return self.__resolved_str_cache__

    @property
    def __forward_code__(self):
        if self.__code__ is not None:
            return self.__code__
        arg = self.__forward_arg__
        try:
            self.__code__ = compile(_rewrite_star_unpack(arg), "<string>", "eval")
        except SyntaxError:
            raise SyntaxError(f"Forward reference must be an expression -- got {arg!r}")
        return self.__code__

    def __eq__(self, other):
        if not isinstance(other, ForwardRef):
            return NotImplemented
        return (
            self.__forward_arg__ == other.__forward_arg__
            and self.__forward_module__ == other.__forward_module__
            # Use "is" here because we use id() for this in __hash__
            # because dictionaries are not hashable.
            and self.__globals__ is other.__globals__
            and self.__forward_is_class__ == other.__forward_is_class__
            # Two separate cells are always considered unequal in forward refs.
            and (
                {name: id(cell) for name, cell in self.__cell__.items()}
                == {name: id(cell) for name, cell in other.__cell__.items()}
                if isinstance(self.__cell__, dict) and isinstance(other.__cell__, dict)
                else self.__cell__ is other.__cell__
            )
            and self.__owner__ == other.__owner__
            and (
                (tuple(sorted(self.__extra_names__.items())) if self.__extra_names__ else None) ==
                (tuple(sorted(other.__extra_names__.items())) if other.__extra_names__ else None)
            )
        )

    def __hash__(self):
        return hash((
            self.__forward_arg__,
            self.__forward_module__,
            id(self.__globals__),  # dictionaries are not hashable, so hash by identity
            self.__forward_is_class__,
            (  # cells are not hashable as well
                tuple(sorted([(name, id(cell)) for name, cell in self.__cell__.items()]))
                if isinstance(self.__cell__, dict) else id(self.__cell__),
            ),
            self.__owner__,
            tuple(sorted(self.__extra_names__.items())) if self.__extra_names__ else None,
        ))

    def __or__(self, other):
        return types.UnionType[self, other]

    def __ror__(self, other):
        return types.UnionType[other, self]

    def __repr__(self):
        extra = []
        if self.__forward_module__ is not None:
            extra.append(f", module={self.__forward_module__!r}")
        if self.__forward_is_class__:
            extra.append(", is_class=True")
        if self.__owner__ is not None:
            extra.append(f", owner={self.__owner__!r}")
        return f"ForwardRef({self.__resolved_str__!r}{''.join(extra)})"


def _rewrite_star_unpack(arg):
    """If the given argument annotation expression is a star unpack e.g. `'*Ts'`
       rewrite it to a valid expression.
       """
    if arg.lstrip().startswith("*"):
        return f"({arg},)[0]"  # E.g. (*Ts,)[0] or (*tuple[int, int],)[0]
    else:
        return arg




# The little syntax tree annotation stand-ins are written down with. The
# runtime does not expose Python's own, so the few node kinds a stand-in can
# become are made here, and written as ast.unparse would write them.
_PRECEDENCE = {'<': 7, '<=': 7, '>': 7, '>=': 7, '==': 7, '!=': 7,
               '|': 9, '^': 10, '&': 11, '<<': 12, '>>': 12, '+': 13, '-': 13,
               '*': 14, '/': 14, '//': 14, '%': 14, '@': 14, '**': 16}
_FACTOR = 15
_ATOM = 18


class _SynNode:
    def precedence(self):
        return _ATOM


class _SynName(_SynNode):
    def __init__(self, id):
        self.id = id


class _SynConstant(_SynNode):
    def __init__(self, value):
        self.value = value


class _SynAttribute(_SynNode):
    def __init__(self, value, attr):
        self.value = value
        self.attr = attr


class _SynSubscript(_SynNode):
    def __init__(self, value, slice):
        self.value = value
        self.slice = slice


class _SynSlice(_SynNode):
    def __init__(self, lower=None, upper=None, step=None):
        self.lower = lower
        self.upper = upper
        self.step = step


class _SynSequence(_SynNode):
    def __init__(self, elts):
        self.elts = elts


class _SynTuple(_SynSequence):
    pass


class _SynList(_SynSequence):
    pass


class _SynSet(_SynSequence):
    pass


class _SynDict(_SynNode):
    def __init__(self, keys, values):
        self.keys = keys
        self.values = values


class _SynStarred(_SynNode):
    def __init__(self, value):
        self.value = value


class _SynKeyword(_SynNode):
    def __init__(self, arg, value):
        self.arg = arg
        self.value = value


class _SynCall(_SynNode):
    def __init__(self, func, args, keywords):
        self.func = func
        self.args = args
        self.keywords = keywords


class _SynBinOp(_SynNode):
    def __init__(self, left, op, right):
        self.left = left
        self.op = op
        self.right = right

    def precedence(self):
        return _PRECEDENCE[self.op]


class _SynCompare(_SynNode):
    def __init__(self, left, ops, comparators):
        self.left = left
        self.ops = ops
        self.comparators = comparators

    def precedence(self):
        return 7


class _SynUnaryOp(_SynNode):
    def __init__(self, op, operand):
        self.op = op
        self.operand = operand

    def precedence(self):
        return _FACTOR


def _syn_operand(node, least):
    text = _syn_unparse(node)
    if node.precedence() < least:
        return '(' + text + ')'
    return text


def _syn_unparse(node):
    if isinstance(node, _SynName):
        return node.id
    if isinstance(node, _SynConstant):
        return '...' if node.value is ... else repr(node.value)
    if isinstance(node, _SynAttribute):
        return _syn_operand(node.value, _ATOM) + '.' + node.attr
    if isinstance(node, _SynSubscript):
        inner = node.slice
        if isinstance(inner, _SynTuple):
            text = ', '.join(_syn_unparse(each) for each in inner.elts)
            if len(inner.elts) == 1:
                text += ','
        else:
            text = _syn_unparse(inner)
        return _syn_operand(node.value, _ATOM) + '[' + text + ']'
    if isinstance(node, _SynSlice):
        text = (_syn_unparse(node.lower) if node.lower is not None else '') + ':'
        if node.upper is not None:
            text += _syn_unparse(node.upper)
        if node.step is not None:
            text += ':' + _syn_unparse(node.step)
        return text
    if isinstance(node, _SynTuple):
        if len(node.elts) == 1:
            return '(' + _syn_unparse(node.elts[0]) + ',)'
        return '(' + ', '.join(_syn_unparse(each) for each in node.elts) + ')'
    if isinstance(node, _SynList):
        return '[' + ', '.join(_syn_unparse(each) for each in node.elts) + ']'
    if isinstance(node, _SynSet):
        return '{' + ', '.join(_syn_unparse(each) for each in node.elts) + '}'
    if isinstance(node, _SynDict):
        return '{' + ', '.join(_syn_unparse(key) + ': ' + _syn_unparse(value)
                               for key, value in zip(node.keys, node.values)) + '}'
    if isinstance(node, _SynStarred):
        return '*' + _syn_operand(node.value, 8)
    if isinstance(node, _SynKeyword):
        return node.arg + '=' + _syn_unparse(node.value)
    if isinstance(node, _SynCall):
        parts = [_syn_unparse(each) for each in node.args]
        parts += [_syn_unparse(each) for each in node.keywords]
        return _syn_operand(node.func, _ATOM) + '(' + ', '.join(parts) + ')'
    if isinstance(node, _SynBinOp):
        mine = _PRECEDENCE[node.op]
        right_first = node.op == '**'
        left = _syn_operand(node.left, mine + 1 if right_first else mine)
        right = _syn_operand(node.right, mine if right_first else mine + 1)
        return left + ' ' + node.op + ' ' + right
    if isinstance(node, _SynCompare):
        text = _syn_operand(node.left, 8)
        for op, other in zip(node.ops, node.comparators):
            text += ' ' + op + ' ' + _syn_operand(other, 8)
        return text
    if isinstance(node, _SynUnaryOp):
        return node.op + _syn_operand(node.operand, _FACTOR)
    raise TypeError('cannot write ' + repr(node))


class _syn:
    AST = _SynNode
    Name = _SynName
    Constant = _SynConstant
    Attribute = _SynAttribute
    Subscript = _SynSubscript
    Slice = _SynSlice
    Tuple = _SynTuple
    List = _SynList
    Set = _SynSet
    Dict = _SynDict
    Starred = _SynStarred
    keyword = _SynKeyword
    Call = _SynCall
    BinOp = _SynBinOp
    Compare = _SynCompare
    UnaryOp = _SynUnaryOp
    unparse = staticmethod(_syn_unparse)
    Add = staticmethod(lambda: '+')
    Sub = staticmethod(lambda: '-')
    Mult = staticmethod(lambda: '*')
    MatMult = staticmethod(lambda: '@')
    Div = staticmethod(lambda: '/')
    Mod = staticmethod(lambda: '%')
    LShift = staticmethod(lambda: '<<')
    RShift = staticmethod(lambda: '>>')
    BitOr = staticmethod(lambda: '|')
    BitXor = staticmethod(lambda: '^')
    BitAnd = staticmethod(lambda: '&')
    FloorDiv = staticmethod(lambda: '//')
    Pow = staticmethod(lambda: '**')
    Lt = staticmethod(lambda: '<')
    LtE = staticmethod(lambda: '<=')
    Eq = staticmethod(lambda: '==')
    NotEq = staticmethod(lambda: '!=')
    Gt = staticmethod(lambda: '>')
    GtE = staticmethod(lambda: '>=')
    Invert = staticmethod(lambda: '~')
    UAdd = staticmethod(lambda: '+')
    USub = staticmethod(lambda: '-')


class _Stringifier:
    # Must match the slots on ForwardRef, so we can turn an instance of one into an
    # instance of the other in place.
    __slots__ = _SLOTS

    def __init__(
        self,
        node,
        globals=None,
        owner=None,
        is_class=False,
        cell=None,
        *,
        stringifier_dict,
        extra_names=None,
    ):
        # Either an AST node or a simple str (for the common case where a ForwardRef
        # represent a single name).
        assert isinstance(node, (_syn.AST, str))
        self.__arg__ = None
        self.__forward_is_argument__ = False
        self.__forward_is_class__ = is_class
        self.__forward_module__ = None
        self.__code__ = None
        self.__ast_node__ = node
        self.__globals__ = globals
        self.__extra_names__ = extra_names
        self.__cell__ = cell
        self.__owner__ = owner
        self.__stringifier_dict__ = stringifier_dict
        self.__resolved_str_cache__ = None  # Needed for ForwardRef

    def __convert_to_ast(self, other):
        if isinstance(other, _Stringifier):
            if isinstance(other.__ast_node__, str):
                return _syn.Name(id=other.__ast_node__), other.__extra_names__
            return other.__ast_node__, other.__extra_names__
        elif (
            # In STRING format we don't bother with the create_unique_name() dance;
            # it's better to emit the repr() of the object instead of an opaque name.
            self.__stringifier_dict__.format == Format.STRING
            or other is None
            or type(other) in (str, int, float, bool, complex)
        ):
            return _syn.Constant(value=other), None
        elif type(other) is dict:
            extra_names = {}
            keys = []
            values = []
            for key, value in other.items():
                new_key, new_extra_names = self.__convert_to_ast(key)
                if new_extra_names is not None:
                    extra_names.update(new_extra_names)
                keys.append(new_key)
                new_value, new_extra_names = self.__convert_to_ast(value)
                if new_extra_names is not None:
                    extra_names.update(new_extra_names)
                values.append(new_value)
            return _syn.Dict(keys, values), extra_names
        elif type(other) in (list, tuple, set):
            extra_names = {}
            elts = []
            for elt in other:
                new_elt, new_extra_names = self.__convert_to_ast(elt)
                if new_extra_names is not None:
                    extra_names.update(new_extra_names)
                elts.append(new_elt)
            ast_class = {list: _syn.List, tuple: _syn.Tuple, set: _syn.Set}[type(other)]
            return ast_class(elts), extra_names
        else:
            name = self.__stringifier_dict__.create_unique_name()
            return _syn.Name(id=name), {name: other}

    def __convert_to_ast_getitem(self, other):
        if isinstance(other, slice):
            extra_names = {}

            def conv(obj):
                if obj is None:
                    return None
                new_obj, new_extra_names = self.__convert_to_ast(obj)
                if new_extra_names is not None:
                    extra_names.update(new_extra_names)
                return new_obj

            return _syn.Slice(
                lower=conv(other.start),
                upper=conv(other.stop),
                step=conv(other.step),
            ), extra_names
        else:
            return self.__convert_to_ast(other)

    def __get_ast(self):
        node = self.__ast_node__
        if isinstance(node, str):
            return _syn.Name(id=node)
        return node

    def __make_new(self, node, extra_names=None):
        new_extra_names = {}
        if self.__extra_names__ is not None:
            new_extra_names.update(self.__extra_names__)
        if extra_names is not None:
            new_extra_names.update(extra_names)
        stringifier = _Stringifier(
            node,
            self.__globals__,
            self.__owner__,
            self.__forward_is_class__,
            stringifier_dict=self.__stringifier_dict__,
            extra_names=new_extra_names or None,
        )
        self.__stringifier_dict__.stringifiers.append(stringifier)
        return stringifier

    # Must implement this since we set __eq__. We hash by identity so that
    # stringifiers in dict keys are kept separate.
    def __hash__(self):
        return id(self)

    def __getitem__(self, other):
        # Special case, to avoid stringifying references to class-scoped variables
        # as '__classdict__["x"]'.
        if self.__ast_node__ == "__classdict__":
            raise KeyError
        if isinstance(other, tuple):
            extra_names = {}
            elts = []
            for elt in other:
                new_elt, new_extra_names = self.__convert_to_ast_getitem(elt)
                if new_extra_names is not None:
                    extra_names.update(new_extra_names)
                elts.append(new_elt)
            other = _syn.Tuple(elts)
        else:
            other, extra_names = self.__convert_to_ast_getitem(other)
        assert isinstance(other, _syn.AST), repr(other)
        return self.__make_new(_syn.Subscript(self.__get_ast(), other), extra_names)

    def __getattr__(self, attr):
        return self.__make_new(_syn.Attribute(self.__get_ast(), attr))

    def __call__(self, *args, **kwargs):
        extra_names = {}
        ast_args = []
        for arg in args:
            new_arg, new_extra_names = self.__convert_to_ast(arg)
            if new_extra_names is not None:
                extra_names.update(new_extra_names)
            ast_args.append(new_arg)
        ast_kwargs = []
        for key, value in kwargs.items():
            new_value, new_extra_names = self.__convert_to_ast(value)
            if new_extra_names is not None:
                extra_names.update(new_extra_names)
            ast_kwargs.append(_syn.keyword(key, new_value))
        return self.__make_new(_syn.Call(self.__get_ast(), ast_args, ast_kwargs), extra_names)

    def __iter__(self):
        yield self.__make_new(_syn.Starred(self.__get_ast()))

    def __repr__(self):
        if isinstance(self.__ast_node__, str):
            return self.__ast_node__
        return _syn.unparse(self.__ast_node__)

    def __format__(self, format_spec):
        raise TypeError("Cannot stringify annotation containing string formatting")

    def _make_binop(op: _syn.AST):
        def binop(self, other):
            rhs, extra_names = self.__convert_to_ast(other)
            return self.__make_new(
                _syn.BinOp(self.__get_ast(), op, rhs), extra_names
            )

        return binop

    __add__ = _make_binop(_syn.Add())
    __sub__ = _make_binop(_syn.Sub())
    __mul__ = _make_binop(_syn.Mult())
    __matmul__ = _make_binop(_syn.MatMult())
    __truediv__ = _make_binop(_syn.Div())
    __mod__ = _make_binop(_syn.Mod())
    __lshift__ = _make_binop(_syn.LShift())
    __rshift__ = _make_binop(_syn.RShift())
    __or__ = _make_binop(_syn.BitOr())
    __xor__ = _make_binop(_syn.BitXor())
    __and__ = _make_binop(_syn.BitAnd())
    __floordiv__ = _make_binop(_syn.FloorDiv())
    __pow__ = _make_binop(_syn.Pow())

    del _make_binop

    def _make_rbinop(op: _syn.AST):
        def rbinop(self, other):
            new_other, extra_names = self.__convert_to_ast(other)
            return self.__make_new(
                _syn.BinOp(new_other, op, self.__get_ast()), extra_names
            )

        return rbinop

    __radd__ = _make_rbinop(_syn.Add())
    __rsub__ = _make_rbinop(_syn.Sub())
    __rmul__ = _make_rbinop(_syn.Mult())
    __rmatmul__ = _make_rbinop(_syn.MatMult())
    __rtruediv__ = _make_rbinop(_syn.Div())
    __rmod__ = _make_rbinop(_syn.Mod())
    __rlshift__ = _make_rbinop(_syn.LShift())
    __rrshift__ = _make_rbinop(_syn.RShift())
    __ror__ = _make_rbinop(_syn.BitOr())
    __rxor__ = _make_rbinop(_syn.BitXor())
    __rand__ = _make_rbinop(_syn.BitAnd())
    __rfloordiv__ = _make_rbinop(_syn.FloorDiv())
    __rpow__ = _make_rbinop(_syn.Pow())

    del _make_rbinop

    def _make_compare(op):
        def compare(self, other):
            rhs, extra_names = self.__convert_to_ast(other)
            return self.__make_new(
                _syn.Compare(
                    left=self.__get_ast(),
                    ops=[op],
                    comparators=[rhs],
                ),
                extra_names,
            )

        return compare

    __lt__ = _make_compare(_syn.Lt())
    __le__ = _make_compare(_syn.LtE())
    __eq__ = _make_compare(_syn.Eq())
    __ne__ = _make_compare(_syn.NotEq())
    __gt__ = _make_compare(_syn.Gt())
    __ge__ = _make_compare(_syn.GtE())

    del _make_compare

    def _make_unary_op(op):
        def unary_op(self):
            return self.__make_new(_syn.UnaryOp(op, self.__get_ast()))

        return unary_op

    __invert__ = _make_unary_op(_syn.Invert())
    __pos__ = _make_unary_op(_syn.UAdd())
    __neg__ = _make_unary_op(_syn.USub())

    del _make_unary_op


class _StringifierDict(dict):
    def __init__(self, namespace, *, globals=None, owner=None, is_class=False, format):
        super().__init__(namespace)
        self.namespace = namespace
        self.globals = globals
        self.owner = owner
        self.is_class = is_class
        self.stringifiers = []
        self.next_id = 1
        self.format = format

    def __missing__(self, key):
        fwdref = _Stringifier(
            key,
            globals=self.globals,
            owner=self.owner,
            is_class=self.is_class,
            stringifier_dict=self,
        )
        self.stringifiers.append(fwdref)
        return fwdref

    def transmogrify(self, cell_dict):
        for obj in self.stringifiers:
            obj.__class__ = ForwardRef
            obj.__stringifier_dict__ = None  # not needed for ForwardRef
            if isinstance(obj.__ast_node__, str):
                obj.__arg__ = obj.__ast_node__
                obj.__ast_node__ = None
            if cell_dict is not None and obj.__cell__ is None:
                obj.__cell__ = cell_dict

    def create_unique_name(self):
        name = f"__annotationlib_name_{self.next_id}__"
        self.next_id += 1
        return name


def annotations_to_string(annotations):
    """Convert an annotation dict containing values to approximately the STRING format.

    Always returns a fresh a dictionary.
    """
    return {
        n: t if isinstance(t, str) else type_repr(t)
        for n, t in annotations.items()
    }
