def adorn(cls):
    return cls

@adorn
class Sample:
    def answer(self):
        return 7

print(Sample().answer())
