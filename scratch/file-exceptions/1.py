def read_classes():
    class Outer(Base, metaclass=Meta):
        class Inner: pass
        def method(self):
            try:
                raise MissingError() from None
            except (MissingError, OtherError) as err:
                raise
            finally:
                pass
print("read classes")
