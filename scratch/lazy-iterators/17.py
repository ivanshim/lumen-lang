class Pull:
    def __init__(self):
        self.busy = False
    def __call__(self):
        if self.busy:
            return 2
        self.busy = True
        list(self.it)
        return 1

p = Pull()
p.it = iter(p, 2)
print(next(p.it, "end"), next(p.it, "end"))
