# tests/python/test_with.py:856
if False:
    def test_exit_called_after_interrupt(self):
        cm = SelfInterruptingContextManager()
        self.assertFalse(cm.within())
        try:
            with cm:
                self.assertTrue(cm.within())
        except KeyboardInterrupt:
            self.assertFalse(cm.within())
            return
        except:
            self.fail("Wrong exception raised")
        self.fail("No exception raised")



print('read')
