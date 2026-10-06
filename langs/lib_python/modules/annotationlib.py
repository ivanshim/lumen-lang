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
        raise NotImplementedError("annotationlib cannot give annotations with fake globals here")

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
    if format == Format.STRING:
        # A function that was written down here keeps the text of each of
        # its annotations; any other annotate function cannot give them.
        try:
            return annotate(Format.STRING)
        except NotImplementedError:
            raise NotImplementedError("annotationlib cannot produce unevaluated annotation strings here")
    try:
        return annotate(Format.VALUE)
    except NameError:
        if format != Format.FORWARDREF:
            raise
    # Some name is not there yet: each annotation is read again from its
    # written form, and the ones that cannot be worked out stand as
    # references to be resolved later.
    try:
        written = annotate(Format.STRING)
    except NotImplementedError:
        written = None
    if written is None:
        return annotate(Format.VALUE)
    scope = getattr(annotate, '__globals__', None)
    result = {}
    for key, text in written.items():
        result[key] = ForwardRef(text, owner=owner).evaluate(globals=scope, format=Format.FORWARDREF)
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

            # An expression with a name still missing stands as one
            # reference to the whole of it: the names inside it are not
            # replaced one by one by symbolic stand-ins here.
            return self

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
