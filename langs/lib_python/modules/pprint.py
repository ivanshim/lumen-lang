# Container punctuation needs distinct tuple and set values. Until those
# are held, printing a list in their stead would conceal a wrong answer.
def pformat(object, indent=1, width=80, depth=None, compact=False, sort_dicts=True, underscore_numbers=False):
    raise 'NotImplementedError: pformat needs distinct container representations'

def pprint(object, stream=None, indent=1, width=80, depth=None, compact=False, sort_dicts=True, underscore_numbers=False):
    text = pformat(object, indent, width, depth, compact, sort_dicts, underscore_numbers)
    if stream is None:
        print(text)
    else:
        stream.write(text + '\n')

class PrettyPrinter:
    def __init__(self, indent=1, width=80, depth=None, stream=None, compact=False, sort_dicts=True, underscore_numbers=False):
        raise 'NotImplementedError: PrettyPrinter needs distinct container representations'
