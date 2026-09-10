def suspended():
    print('must not run')
    try:
        yield 1
    finally:
        print('must not run')

suspended()
