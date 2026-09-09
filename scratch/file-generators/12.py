def context():
    print('must not run')

with context() as item:
    print('must not run')
