if False:
    @decorate
    class C[T: (int, str), *Ts, **P](base(), metaclass=choose(), **options):
        field: Missing = 1
        @decorate
        def method[U](self, value: U) -> U:
            return value
        class Nested:
            pass
    class Empty(): pass
    class Defaulted[T = int]: pass
print("read classes")
