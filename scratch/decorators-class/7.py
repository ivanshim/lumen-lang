def mark(c):
    print('class')
    return c
class Outer:
    @mark
    class Inner:
        @staticmethod
        def value():
            return 8
print(Outer.Inner.value())
