# tests/python/test_with.py:72
if False:
    def __enter__(self):
        if self.entered is not None:
            raise RuntimeError("Context is not reentrant")
        self.entered = deque()
        vars = []
        try:
            for mgr in self.managers:
                vars.append(mgr.__enter__())
                self.entered.appendleft(mgr)
        except:
            if not self.__exit__(*sys.exc_info()):
                raise
        return vars


print('read')
