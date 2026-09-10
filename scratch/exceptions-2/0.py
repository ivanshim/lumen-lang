try:
    raise ExceptionGroup("g", [ValueError(1), TypeError(2)])
except* ValueError as eg:
    print(type(eg).__name__, repr([type(e).__name__ for e in eg.exceptions]))
except* TypeError as eg:
    print("T", len(eg.exceptions))
