match {"key": 1}:
    case {"key": found}:
        pass
    case Shape(key=found):
        pass
    case (a, b):
        pass
    case _ as rest:
        pass
