class S(str):
    def shout(self): return self.upper() + "!"
s = S("ab")
print(s, len(s), s.shout(), isinstance(s, str), type(s).__name__)
