with (manager() as first, manager() as second):
    try:
        raise MissingError
    finally:
        pass
