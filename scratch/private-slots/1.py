def case(name, f):
    try:
        print(name, f())
    except Exception as e:
        print(name, type(e).__name__, str(e))
class Rat:
    __slots__ = ['_Rat__num', '_Rat__den']
    def __init__(self):
        self.__num = 3
        self.__den = 4
    def run(self):
        self.__num += 2
        value = (self.__num, self.__den)
        del self.__den
        return value, hasattr(self, '_Rat__den')
case('explicit slots', lambda: Rat().run())
class __Hidden:
    __slots__ = ['__x', '__tail__', '_single', '___more']
    def run(self):
        self.__x = 7
        self.__tail__ = 8
        self._single = 9
        self.___more = 10
        def nested():
            return self.__x
        return nested(), self.__tail__, self._single, self.___more, hasattr(self, '_Hidden__x'), hasattr(self, '__x')
case('private slots', lambda: __Hidden().run())
class ___:
    __slots__ = ['__x']
    def run(self):
        self.__x = 11
        return self.__x, hasattr(self, '__x')
case('underscore class', lambda: ___().run())
class Names:
    __x = 12
    copied = __x
    __gone = 0
    del __gone
    def __method(self, __arg):
        __local = __arg + 1
        def inner():
            return __local
        return inner()
    def run(self):
        return self.__method(13), self.__x
case('identifiers', lambda: (Names.copied, Names().run(), hasattr(Names, '_Names__gone')))
case('method name', lambda: Names._Names__method.__name__)
case('keyword parameter', lambda: Names()._Names__method(_Names__arg=14))
class Outer:
    __x = 21
    class __Inner:
        __x = 22
        def run(self):
            return self.__x
    def run(self):
        return self.__x, self.__Inner().run()
case('nested class', lambda: (Outer().run(), Outer._Outer__Inner.__name__))
class Base:
    __slots__ = ['__x']
    def put(self):
        self.__x = 31
    def get(self):
        return self.__x
class Child(Base):
    __slots__ = ['__x']
    def run(self):
        self.put()
        self.__x = 32
        return self.get(), self.__x
case('inheritance', lambda: Child().run())
class Dynamic:
    def run(self):
        setattr(self, '__x', 41)
        self.__x = 42
        return getattr(self, '__x'), self.__x, f'{self.__x}'
case('literal strings', lambda: Dynamic().run())
class Conflict:
    pass
def conflict():
    class Clash:
        __slots__ = ['__x']
        __x = 1
case('slot conflict', conflict)
def keywords(**kw):
    return sorted(kw)
class Call:
    def run(self):
        return keywords(__x=1)
case('call keywords', lambda: Call().run())
class Inline: __x = 51; copied = __x
case('inline class', lambda: (Inline.copied, hasattr(Inline, '_Inline__x')))
class Extended:
    __value = 61
    values = [__i for __i in (1, 2)]
    def run(self, __arg=62):
        __local = 63
        def __inner():
            nonlocal __local
            __local += 1
            return __local
        import math as __math
        from sys import version_info as __version
        return __arg, __inner(), __inner.__name__, __math.sqrt(4), __version[0]
case('nested bindings', lambda: (Extended.values, Extended().run()))
_Global__value = 70
class Global:
    def run(self):
        global __value
        __value += 1
        return __value
case('global binding', lambda: (Global().run(), _Global__value))
class Delete:
    def run(self):
        self.__x = 81
        del self.__x
        try:
            return self.__x
        except AttributeError:
            return hasattr(self, '_Delete__x')
case('delete attribute', lambda: Delete().run())
class ____Suffix:
    __slots__ = ['__x_', '__x__', '___x', 'normal']
    def run(self):
        self.__x_ = 91
        self.__x__ = 92
        self.___x = 93
        return self.__x_, self.__x__, self.___x, hasattr(self, '_Suffix__x_'), self.__slots__
case('suffix rules', lambda: ____Suffix().run())
Made = type('__Made', (), {'__slots__': ['__x']})
def made():
    obj = Made()
    obj._Made__x = 101
    return obj._Made__x, hasattr(obj, '__x'), Made.__slots__
case('dynamic slots', made)
class Surround:
    __base = Base
    class Inner(__base):
        def run(self):
            self.put()
            return self.get()
case('nested bases', lambda: Surround.Inner().run())
__outside = 111
case('outside unchanged', lambda: __outside)
