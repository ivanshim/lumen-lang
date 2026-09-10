def unread_until_called():
    @outer('first', size=42)
    @inner(*args, **kwds)
    class C(object):
        @staticmethod
        @another(value=1)
        def method(*args, **kwds):
            return 42

print('class and method bodies read')
