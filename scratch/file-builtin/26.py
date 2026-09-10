class Bounded[T: (int, str) = int]:
    def method(self, value: T):
        return value

class Variadic[*Ts, **P]:
    pass
