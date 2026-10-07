# Native annotation records retain expression closures. VALUE evaluates them;
# FORWARDREF rebuilds their namespace with symbolic missing-name objects.
# The public reader refuses unsupported formats rather than inventing strings.

class Format:
    VALUE = 1
    VALUE_WITH_FAKE_GLOBALS = 2
    FORWARDREF = 3
    STRING = 4

_FORMAT_NAMES = {1: 'VALUE', 2: 'VALUE_WITH_FAKE_GLOBALS', 3: 'FORWARDREF', 4: 'STRING'}

def _check_format(format):
    if format not in _FORMAT_NAMES:
        raise 'ValueError: ' + str(format) + ' is not a valid Format'
    if format not in (Format.VALUE, Format.FORWARDREF):
        raise NotImplementedError("annotationlib cannot produce unevaluated annotation strings here")

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
    try:
        return annotate(format)
    except NotImplementedError:
        if format != Format.FORWARDREF:
            raise
        # Native annotation functions support VALUE; resolved expressions
        # have the same result in FORWARDREF format.
        return annotate(Format.VALUE)

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
    return dict(stored)

# Runtime adapter derived from CPython v3.14.8 / 8e6e75d9102e, Lib/annotationlib.py; PSF License.
# ForwardRef and symbolic namespace protocols use retained native syntax nodes.
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
            raise NotImplementedError('symbolic forward-reference expressions are not supported')

    @property
    def __forward_arg__(self):
        if self.__arg__ is not None:
            return self.__arg__
        if self.__ast_node__ is not None:
            self.__arg__ = ast.unparse(self.__ast_node__)
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
                visitor = _ExtraNameFixer(names)
                ast_expr = ast.parse(resolved_str, mode="eval").body
                node = visitor.visit(ast_expr)
                resolved_str = ast.unparse(node)

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

# Symbolic annotation namespaces, derived from CPython v3.14.8; PSF License.
# Template objects remain unavailable; other expression protocols are retained.


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
        assert isinstance(node, (ast.AST, str))
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
                return ast.Name(id=other.__ast_node__), other.__extra_names__
            return other.__ast_node__, other.__extra_names__
        elif (
            # In STRING format we don't bother with the create_unique_name() dance;
            # it's better to emit the repr() of the object instead of an opaque name.
            self.__stringifier_dict__.format == Format.STRING
            or other is None
            or type(other) in (str, int, float, bool, complex)
        ):
            return ast.Constant(value=other), None
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
            return ast.Dict(keys, values), extra_names
        elif type(other) in (list, tuple, set):
            extra_names = {}
            elts = []
            for elt in other:
                new_elt, new_extra_names = self.__convert_to_ast(elt)
                if new_extra_names is not None:
                    extra_names.update(new_extra_names)
                elts.append(new_elt)
            ast_class = {list: ast.List, tuple: ast.Tuple, set: ast.Set}[type(other)]
            return ast_class(elts), extra_names
        else:
            name = self.__stringifier_dict__.create_unique_name()
            return ast.Name(id=name), {name: other}

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

            return ast.Slice(
                lower=conv(other.start),
                upper=conv(other.stop),
                step=conv(other.step),
            ), extra_names
        else:
            return self.__convert_to_ast(other)

    def __get_ast(self):
        node = self.__ast_node__
        if isinstance(node, str):
            return ast.Name(id=node)
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
            other = ast.Tuple(elts)
        else:
            other, extra_names = self.__convert_to_ast_getitem(other)
        assert isinstance(other, ast.AST), repr(other)
        return self.__make_new(ast.Subscript(self.__get_ast(), other), extra_names)

    def __getattr__(self, attr):
        return self.__make_new(ast.Attribute(self.__get_ast(), attr))

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
            ast_kwargs.append(ast.keyword(key, new_value))
        return self.__make_new(ast.Call(self.__get_ast(), ast_args, ast_kwargs), extra_names)

    def __iter__(self):
        yield self.__make_new(ast.Starred(self.__get_ast()))

    def __repr__(self):
        if isinstance(self.__ast_node__, str):
            return self.__ast_node__
        return ast.unparse(self.__ast_node__)

    def __format__(self, format_spec):
        raise TypeError("Cannot stringify annotation containing string formatting")

    def _make_binop(op: ast.AST):
        def binop(self, other):
            rhs, extra_names = self.__convert_to_ast(other)
            return self.__make_new(
                ast.BinOp(self.__get_ast(), op, rhs), extra_names
            )

        return binop

    __add__ = _make_binop(ast.Add())
    __sub__ = _make_binop(ast.Sub())
    __mul__ = _make_binop(ast.Mult())
    __matmul__ = _make_binop(ast.MatMult())
    __truediv__ = _make_binop(ast.Div())
    __mod__ = _make_binop(ast.Mod())
    __lshift__ = _make_binop(ast.LShift())
    __rshift__ = _make_binop(ast.RShift())
    __or__ = _make_binop(ast.BitOr())
    __xor__ = _make_binop(ast.BitXor())
    __and__ = _make_binop(ast.BitAnd())
    __floordiv__ = _make_binop(ast.FloorDiv())
    __pow__ = _make_binop(ast.Pow())

    del _make_binop

    def _make_rbinop(op: ast.AST):
        def rbinop(self, other):
            new_other, extra_names = self.__convert_to_ast(other)
            return self.__make_new(
                ast.BinOp(new_other, op, self.__get_ast()), extra_names
            )

        return rbinop

    __radd__ = _make_rbinop(ast.Add())
    __rsub__ = _make_rbinop(ast.Sub())
    __rmul__ = _make_rbinop(ast.Mult())
    __rmatmul__ = _make_rbinop(ast.MatMult())
    __rtruediv__ = _make_rbinop(ast.Div())
    __rmod__ = _make_rbinop(ast.Mod())
    __rlshift__ = _make_rbinop(ast.LShift())
    __rrshift__ = _make_rbinop(ast.RShift())
    __ror__ = _make_rbinop(ast.BitOr())
    __rxor__ = _make_rbinop(ast.BitXor())
    __rand__ = _make_rbinop(ast.BitAnd())
    __rfloordiv__ = _make_rbinop(ast.FloorDiv())
    __rpow__ = _make_rbinop(ast.Pow())

    del _make_rbinop

    def _make_compare(op):
        def compare(self, other):
            rhs, extra_names = self.__convert_to_ast(other)
            return self.__make_new(
                ast.Compare(
                    left=self.__get_ast(),
                    ops=[op],
                    comparators=[rhs],
                ),
                extra_names,
            )

        return compare

    __lt__ = _make_compare(ast.Lt())
    __le__ = _make_compare(ast.LtE())
    __eq__ = _make_compare(ast.Eq())
    __ne__ = _make_compare(ast.NotEq())
    __gt__ = _make_compare(ast.Gt())
    __ge__ = _make_compare(ast.GtE())

    del _make_compare

    def _make_unary_op(op):
        def unary_op(self):
            return self.__make_new(ast.UnaryOp(op, self.__get_ast()))

        return unary_op

    __invert__ = _make_unary_op(ast.Invert())
    __pos__ = _make_unary_op(ast.UAdd())
    __neg__ = _make_unary_op(ast.USub())

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


def _build_closure(annotate, owner, is_class, stringifier_dict, *, allow_evaluation):
    if not annotate.__closure__:
        return None, None
    new_closure = []
    cell_dict = {}
    for name, cell in zip(annotate.__code__.co_freevars, annotate.__closure__, strict=True):
        cell_dict[name] = cell
        new_cell = None
        if allow_evaluation:
            try:
                cell.cell_contents
            except ValueError:
                pass
            else:
                new_cell = cell
        if new_cell is None:
            fwdref = _Stringifier(
                name,
                cell=cell,
                owner=owner,
                globals=annotate.__globals__,
                is_class=is_class,
                stringifier_dict=stringifier_dict,
            )
            stringifier_dict.stringifiers.append(fwdref)
            new_cell = types.CellType(fwdref)
        new_closure.append(new_cell)
    return tuple(new_closure), cell_dict


def _stringify_single(anno):
    if anno is ...:
        return "..."
    # We have to handle str specially to support PEP 563 stringified annotations.
    elif isinstance(anno, str):
        return anno
    else:
        return repr(anno)


def call_annotate_function(annotate, format, *, owner=None, _is_evaluate=False):
    """Call an __annotate__ function. __annotate__ functions are normally
    generated by the compiler to defer the evaluation of annotations. They
    can be called with any of the format arguments in the Format enum, but
    compiler-generated __annotate__ functions only support the VALUE format.
    This function provides additional functionality to call __annotate__
    functions with the FORWARDREF and STRING formats.

    *annotate* must be an __annotate__ function, which takes a single argument
    and returns a dict of annotations.

    *format* must be a member of the Format enum or one of the corresponding
    integer values.

    *owner* can be the object that owns the annotations (i.e., the module,
    class, or function that the __annotate__ function derives from). With the
    FORWARDREF format, it is used to provide better evaluation capabilities
    on the generated ForwardRef objects.

    """
    if format == Format.VALUE_WITH_FAKE_GLOBALS:
        raise ValueError("The VALUE_WITH_FAKE_GLOBALS format is for internal use only")
    try:
        return annotate(format)
    except NotImplementedError:
        if format == Format.FORWARDREF and isinstance(owner, type) and not isinstance(annotate, types.FunctionType):
            rows = getattr(owner, '__native_annotation_rows__', None)
            if rows is not None:
                return _native_annotation_rows(rows, owner)
    if format == Format.STRING:
        # STRING is implemented by calling the annotate function in a special
        # environment where every name lookup results in an instance of _Stringifier.
        # _Stringifier supports every dunder operation and returns a new _Stringifier.
        # At the end, we get a dictionary that mostly contains _Stringifier objects (or
        # possibly constants if the annotate function uses them directly). We then
        # convert each of those into a string to get an approximation of the
        # original source.

        # Attempt to call with VALUE_WITH_FAKE_GLOBALS to check if it is implemented
        # See: https://github.com/python/cpython/issues/138764
        # Only fail on NotImplementedError
        try:
            annotate(Format.VALUE_WITH_FAKE_GLOBALS)
        except NotImplementedError:
            # Both STRING and VALUE_WITH_FAKE_GLOBALS are not implemented: fallback to VALUE
            return annotations_to_string(annotate(Format.VALUE))
        except Exception:
            pass

        globals = _StringifierDict({}, format=format)
        is_class = isinstance(owner, type)
        closure, _ = _build_closure(
            annotate, owner, is_class, globals, allow_evaluation=False
        )
        func = types.FunctionType(
            annotate.__code__,
            globals,
            closure=closure,
            argdefs=annotate.__defaults__,
            kwdefaults=annotate.__kwdefaults__,
        )
        annos = func(Format.VALUE_WITH_FAKE_GLOBALS)
        if _is_evaluate:
            return _stringify_single(annos)
        return {
            key: _stringify_single(val)
            for key, val in annos.items()
        }
    elif format == Format.FORWARDREF:
        # FORWARDREF is implemented similarly to STRING, but there are two changes,
        # at the beginning and the end of the process.
        # First, while STRING uses an empty dictionary as the namespace, so that all
        # name lookups result in _Stringifier objects, FORWARDREF uses the globals
        # and builtins, so that defined names map to their real values.
        # Second, instead of returning strings, we want to return either real values
        # or ForwardRef objects. To do this, we keep track of all _Stringifier objects
        # created while the annotation is being evaluated, and at the end we convert
        # them all to ForwardRef objects by assigning to __class__. To make this
        # technique work, we have to ensure that the _Stringifier and ForwardRef
        # classes share the same attributes.
        # We use this technique because while the annotations are being evaluated,
        # we want to support all operations that the language allows, including even
        # __getattr__ and __eq__, and return new _Stringifier objects so we can accurately
        # reconstruct the source. But in the dictionary that we eventually return, we
        # want to return objects with more user-friendly behavior, such as an __eq__
        # that returns a bool and an defined set of attributes.
        namespace = {**annotate.__builtins__, **annotate.__globals__}
        is_class = isinstance(owner, type)
        globals = _StringifierDict(
            namespace,
            globals=annotate.__globals__,
            owner=owner,
            is_class=is_class,
            format=format,
        )
        closure, cell_dict = _build_closure(
            annotate, owner, is_class, globals, allow_evaluation=True
        )
        func = types.FunctionType(
            annotate.__code__,
            globals,
            closure=closure,
            argdefs=annotate.__defaults__,
            kwdefaults=annotate.__kwdefaults__,
        )
        try:
            result = func(Format.VALUE_WITH_FAKE_GLOBALS)
        except NotImplementedError:
            # FORWARDREF and VALUE_WITH_FAKE_GLOBALS not supported, fall back to VALUE
            return annotate(Format.VALUE)
        except Exception:
            pass
        else:
            globals.transmogrify(cell_dict)
            return result

        # Try again, but do not provide any globals. This allows us to return
        # a value in certain cases where an exception gets raised during evaluation.
        globals = _StringifierDict(
            {},
            globals=annotate.__globals__,
            owner=owner,
            is_class=is_class,
            format=format,
        )
        closure, cell_dict = _build_closure(
            annotate, owner, is_class, globals, allow_evaluation=False
        )
        func = types.FunctionType(
            annotate.__code__,
            globals,
            closure=closure,
            argdefs=annotate.__defaults__,
            kwdefaults=annotate.__kwdefaults__,
        )
        result = func(Format.VALUE_WITH_FAKE_GLOBALS)
        globals.transmogrify(cell_dict)
        if _is_evaluate:
            if isinstance(result, ForwardRef):
                return result.evaluate(format=Format.FORWARDREF)
            else:
                return result
        else:
            return {
                key: (
                    val.evaluate(format=Format.FORWARDREF)
                    if isinstance(val, ForwardRef)
                    else val
                )
                for key, val in result.items()
            }
    elif format == Format.VALUE:
        # Should be impossible because __annotate__ functions must not raise
        # NotImplementedError for this format.
        raise RuntimeError("annotate function does not support VALUE format")
    else:
        raise ValueError(f"Invalid format: {format!r}")


def annotations_to_string(annotations):
    """Convert an annotation dict containing values to approximately the STRING format.

    Always returns a fresh a dictionary.
    """
    return {
        n: t if isinstance(t, str) else type_repr(t)
        for n, t in annotations.items()
    }

# Native class annotations retain separate zero-argument expression closures.
# Apply the same symbolic namespace and closure protocol as call_annotate_function.
def _native_annotation_rows(rows, owner):
    result = {}
    for index in range(0, len(rows), 2):
        key, expression = rows[index:index + 2]
        if not isinstance(expression, types.FunctionType):
            result[key] = expression
            continue
        try:
            result[key] = expression()
        except NameError:
            namespace = {**expression.__builtins__, **expression.__globals__}
            symbols = _StringifierDict(namespace, globals=expression.__globals__,
                                       owner=owner, is_class=isinstance(owner, type),
                                       format=Format.FORWARDREF)
            closure, cells = _build_closure(expression, owner, isinstance(owner, type),
                                          symbols, allow_evaluation=True)
            rebuilt = types.FunctionType(expression.__code__, symbols, closure=closure)
            result[key] = rebuilt()
            symbols.transmogrify(cells)
    return result

_sentinel = object()

def type_repr(value):
    """Convert a Python value to a format suitable for use with the STRING format.

    This is intended as a helper for tools that support the STRING format but do
    not have access to the code that originally produced the annotations. It uses
    repr() for most objects.

    """
    if isinstance(value, (type, types.FunctionType, types.BuiltinFunctionType)):
        if value.__module__ == "builtins":
            return value.__qualname__
        return f"{value.__module__}.{value.__qualname__}"
    if value is ...:
        return "..."
    return repr(value)


class _ExtraNameFixer(ast.NodeTransformer):
    """Fixer for __extra_names__ items in ForwardRef __repr__ and string evaluation"""
    def __init__(self, extra_names):
        self.extra_names = extra_names

    def visit_Name(self, node: ast.Name):
        if (new_name := self.extra_names.get(node.id, _sentinel)) is not _sentinel:
            node = ast.Name(id=type_repr(new_name))
        return node
