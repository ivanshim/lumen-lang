class Outer:
    # Read the enclosing method's class cell from a nested class body.
    def nested(self):
        class Inner:
            captured = __class__
        return Inner
    # A class body assignment shadows the hidden cell for later reads.
    def shadowed(self):
        class Inner:
            __class__ = 'local'
            captured = __class__
        return Inner
assert Outer().nested().captured is Outer
assert Outer().shadowed().captured == 'local'
print('class-cell lookup preserves enclosing and shadowed bindings')
