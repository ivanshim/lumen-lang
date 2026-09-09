def keep(f):
    return f
class C:
    @keep
    def __init__(self):
        self.x = 1
C()
