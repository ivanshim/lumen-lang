import traceback
import sys
from types import TracebackType
traceback.clear_frames(None)
class Frame:
    def __init__(self): self.calls = 0
    def clear(self): self.calls += 1; raise RuntimeError('active')
class Link:
    def __init__(self, frame): self.tb_frame = frame; self.tb_next = None
frame = Frame()
traceback.clear_frames(Link(frame))
assert frame.calls == 1
def check_live(marker):
    running = sys._getframe()
    assert running.f_locals['marker'] is marker
    traceback.clear_frames(TracebackType(None, running, 0, running.f_lineno))
    assert running.f_locals['marker'] is marker
check_live(frame)
print('native frame cleanup preserves live frames and duck traceback behavior')
