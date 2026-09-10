try:
    [1][5]
except (KeyError, IndexError) as e:
    print(type(e).__name__)
try:
    {}["k"]
except (KeyError, IndexError) as e:
    print(type(e).__name__)
try:
    int("x")
except ValueError as e:
    print(type(e).__name__)
try:
    absent_name
except NameError as e:
    print(type(e).__name__)
