def decorate(func):
    func.__dict__['author'] = 'Cleese'
    return func

def f():
    return 42

decorate(f)
