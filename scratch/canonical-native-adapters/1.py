import http.server
import runpy
import signal
import threading
from test.support import script_helper
from unittest import mock
for module in [http.server,runpy,signal,threading,script_helper,mock]:
    assert module.__file__.endswith(module.__name__.replace('.','/')+'.py')
assert callable(script_helper.assert_python_ok)
assert callable(runpy._run_module_as_main)
m=mock.Mock(return_value=42)
assert m('value')==42
assert m.call_count==1
lock=threading.RLock()
assert lock.acquire()
lock.release()
print('six native adapters retain canonical module locations and runtime behavior')
