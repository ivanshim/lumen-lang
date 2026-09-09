class Generic[T: (int, str) = int, *Ts, **P]:
    def method(self, value: T):
        return value
