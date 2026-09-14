# Names for the kinds of thing a program builds, and the few small
# kinds that can be built here in Python.
#
# Stub: the reader hands out no handle on a function, a bound method, a
# code body, a frame or a generator -- type() of one refuses -- so the
# names for those kinds below stand alone. They can be asked after and
# told apart from one another, but type(x) never answers with one, and
# isinstance against one is false.

NoneType = type(None)


class FunctionType:
    pass


LambdaType = FunctionType


class CodeType:
    pass


class BuiltinFunctionType:
    pass


BuiltinMethodType = BuiltinFunctionType


class WrapperDescriptorType:
    pass


class MethodWrapperType:
    pass


class MethodDescriptorType:
    pass


class ClassMethodDescriptorType:
    pass


class GetSetDescriptorType:
    pass


class MemberDescriptorType:
    pass


class GeneratorType:
    pass


class CoroutineType:
    pass


class AsyncGeneratorType:
    pass


class FrameType:
    pass


class TracebackType:
    pass


class EllipsisType:
    pass


class NotImplementedType:
    pass


class UnionType:
    pass


# A cell holds one value, or none at all, and reading an empty one is an
# error rather than an answer.
class CellType:
    def __init__(self, *contents):
        if len(contents) > 1:
            raise TypeError('cell expected at most 1 argument, got ' + str(len(contents)))
        self._contents = list(contents)

    @property
    def cell_contents(self):
        if len(self._contents) == 0:
            raise ValueError('Cell is empty')
        return self._contents[0]

    def __repr__(self):
        if len(self._contents) == 0:
            return '<cell: empty>'
        return '<cell: ' + repr(self._contents[0]) + '>'


# A function and the thing it is bound to, called as one.
class MethodType:
    def __init__(self, function, instance):
        self.__func__ = function
        self.__self__ = instance

    def __call__(self, *args, **keywords):
        return self.__func__(self.__self__, *args, **keywords)

    def __eq__(self, other):
        if isinstance(other, MethodType):
            return self.__func__ == other.__func__ and self.__self__ is other.__self__
        return NotImplemented

    def __repr__(self):
        return '<bound method of ' + repr(self.__self__) + '>'


# A place to hang names on, which is all a module is from here.
class ModuleType:
    def __init__(self, name, doc=None):
        self.__name__ = name
        self.__doc__ = doc

    def __repr__(self):
        return "<module '" + str(self.__name__) + "'>"


# A reading of a mapping that cannot be written through.
class MappingProxyType:
    def __init__(self, mapping):
        self._mapping = mapping

    def __getitem__(self, key):
        return self._mapping[key]

    def __len__(self):
        return len(self._mapping)

    def __iter__(self):
        return iter(list(self._mapping))

    def __contains__(self, key):
        return key in self._mapping

    def get(self, key, default=None):
        if key in self._mapping:
            return self._mapping[key]
        return default

    def keys(self):
        return list(self._mapping)

    def values(self):
        gathered = []
        for key in list(self._mapping):
            gathered.append(self._mapping[key])
        return gathered

    def items(self):
        gathered = []
        for key in list(self._mapping):
            gathered.append((key, self._mapping[key]))
        return gathered

    def copy(self):
        copied = {}
        for key in list(self._mapping):
            copied[key] = self._mapping[key]
        return copied

    def __eq__(self, other):
        if isinstance(other, MappingProxyType):
            return self.copy() == other.copy()
        return self.copy() == other

    def __repr__(self):
        return 'mappingproxy(' + repr(self._mapping) + ')'


# A bag of named values, compared by the names it carries.
class SimpleNamespace:
    def __init__(self, **keywords):
        for name in list(keywords):
            setattr(self, name, keywords[name])

    def _fields(self):
        gathered = {}
        for name in list(self.__dict__):
            gathered[name] = self.__dict__[name]
        return gathered

    def __eq__(self, other):
        if isinstance(other, SimpleNamespace):
            return self._fields() == other._fields()
        return NotImplemented

    def __ne__(self, other):
        answer = self.__eq__(other)
        if answer is NotImplemented:
            return answer
        return not answer

    def __repr__(self):
        fields = self._fields()
        shown = []
        for name in sorted(list(fields)):
            shown.append(name + '=' + repr(fields[name]))
        return 'namespace(' + ', '.join(shown) + ')'


# What a kind is called. A builtin kind answers to __name__ but is not
# found by getattr, so the name is asked for directly.
def _kind_name(kind):
    try:
        return str(kind.__name__)
    except BaseException:
        return repr(kind)


# A kind named with the kinds it was given, as list[int] is.
class GenericAlias:
    def __init__(self, origin, args):
        self.__origin__ = origin
        if isinstance(args, tuple):
            self.__args__ = args
        else:
            self.__args__ = (args,)

    def __call__(self, *args, **keywords):
        return self.__origin__(*args, **keywords)

    def __eq__(self, other):
        if isinstance(other, GenericAlias):
            return self.__origin__ is other.__origin__ and self.__args__ == other.__args__
        return NotImplemented

    def __repr__(self):
        shown = []
        for given in self.__args__:
            shown.append(_kind_name(given))
        return _kind_name(self.__origin__) + '[' + ', '.join(shown) + ']'


# A member that answers one way on an instance and another on the class.
class DynamicClassAttribute:
    def __init__(self, fget=None, fset=None, fdel=None, doc=None):
        self.fget = fget
        self.fset = fset
        self.fdel = fdel
        self.__doc__ = doc if doc is not None else getattr(fget, '__doc__', None)
        self.overwrite_doc = doc is None

    def __get__(self, instance, owner=None):
        if instance is None:
            raise AttributeError('dynamic class attribute')
        if self.fget is None:
            raise AttributeError('unreadable attribute')
        return self.fget(instance)

    def __set__(self, instance, value):
        if self.fset is None:
            raise AttributeError("can't set attribute")
        self.fset(instance, value)

    def __delete__(self, instance):
        if self.fdel is None:
            raise AttributeError("can't delete attribute")
        self.fdel(instance)

    def getter(self, fget):
        return DynamicClassAttribute(fget, self.fset, self.fdel, self.__doc__)

    def setter(self, fset):
        return DynamicClassAttribute(self.fget, fset, self.fdel, self.__doc__)

    def deleter(self, fdel):
        return DynamicClassAttribute(self.fget, self.fset, fdel, self.__doc__)


__all__ = ['NoneType', 'FunctionType', 'LambdaType', 'CodeType',
           'CellType', 'MethodType', 'BuiltinFunctionType',
           'BuiltinMethodType', 'WrapperDescriptorType',
           'MethodWrapperType', 'MethodDescriptorType',
           'ClassMethodDescriptorType', 'GetSetDescriptorType',
           'MemberDescriptorType', 'GeneratorType', 'CoroutineType',
           'AsyncGeneratorType', 'FrameType', 'TracebackType',
           'EllipsisType', 'NotImplementedType', 'UnionType',
           'ModuleType', 'MappingProxyType', 'SimpleNamespace',
           'GenericAlias', 'DynamicClassAttribute']
