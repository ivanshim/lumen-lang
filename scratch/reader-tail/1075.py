# tests/python/test_grammar.py:1864
if False:
    def test_with_statement(self):
        class manager(object):
            def __enter__(self):
                return (1, 2)
            def __exit__(self, *args):
                pass

        with manager():
            pass
        with manager() as x:
            pass
        with manager() as (x, y):
            pass
        with manager(), manager():
            pass
        with manager() as x, manager() as y:
            pass
        with manager() as x, manager():
            pass

        with (
            manager()
        ):
            pass

        with (
            manager() as x
        ):
            pass

        with (
            manager() as (x, y),
            manager() as z,
        ):
            pass

        with (
            manager(),
            manager()
        ):
            pass

        with (
            manager() as x,
            manager() as y
        ):
            pass

        with (
            manager() as x,
            manager()
        ):
            pass

        with (
            manager() as x,
            manager() as y,
            manager() as z,
        ):
            pass

        with (
            manager() as x,
            manager() as y,
            manager(),
        ):
            pass


print('read')
