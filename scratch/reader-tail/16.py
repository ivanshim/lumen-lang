# test_with.py:673 and 692, indexed answers as context targets.
def reading():
    with manager() as list(targets.values())[0][1]:
        pass
    with manager() as (list(targets.values())[0][2], list(targets.values())[0][1]):
        pass
print('read')
