def decorate(func):
    func.__dict__['author'] = 'Cleese'
    return func

print('attribute target read')
