if False:
    dispatch = {
        (False, False, False):
            lambda args, sep, end, file: print(*args),
        (False, False, True):
            lambda args, sep, end, file: print(file=file, *args),
        (False, True,  False):
            lambda args, sep, end, file: print(end=end, *args),
        (False, True,  True):
            lambda args, sep, end, file: print(end=end, file=file, *args),
        (True,  False, False):
            lambda args, sep, end, file: print(sep=sep, *args),
        (True,  False, True):
            lambda args, sep, end, file: print(sep=sep, file=file, *args),
        (True,  True,  False):
            lambda args, sep, end, file: print(sep=sep, end=end, *args),
        (True,  True,  True):
            lambda args, sep, end, file: print(sep=sep, end=end, file=file, *args),
    }
    
    
print("read")
