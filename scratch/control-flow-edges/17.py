try:
    try:
        raise ValueError("body")
    except ValueError:
        missing_name
except NameError as e:
    print(type(e.__context__).__name__)
