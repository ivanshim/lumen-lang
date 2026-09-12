import sys
class Closed:
    def write(self, text):
        raise ValueError("closed")
sys.stdout = Closed()
try:
    print("lost")
except ValueError as error:
    print(str(error), file=sys.__stderr__)
sys.stdout = sys.__stdout__
