import pathlib
import tempfile
import unittest
import dispatch
import workspace_adapter as ws

class RuntimeTests(unittest.TestCase):
    def test_registry(self):
        self.assertEqual(len(dispatch.registry()),20)
    def test_dispatch(self):
        self.assertEqual(dispatch.dispatch("math.calculate",{"operation":"add","a":2,"b":3}),{"ok":True,"result":{"value":5}})
        with self.assertRaises(ValueError): dispatch.dispatch("math.calculate",{"operation":"add","a":True,"b":3})
        with self.assertRaises(KeyError): dispatch.dispatch("shell.run",{})
    def test_workspace(self):
        with tempfile.TemporaryDirectory() as d:
            p=pathlib.Path(d)
            (p/"hello.txt").write_bytes(b"hello")
            (p/".env").write_text("secret")
            self.assertEqual(ws.inspect(p,"hello.txt","metadata")["bytes"],5)
            self.assertEqual(ws.inspect(p,"hello.txt","sha256")["sha256"],"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
            self.assertEqual(ws.inspect(p,".","list")["entries"],[{"name":"hello.txt","kind":"file"}])
            for name in ("../outside","/etc/passwd",".env","C:/file"):
                with self.assertRaises(ValueError): ws.resolve(p,name)
    def test_link(self):
        with tempfile.TemporaryDirectory() as d:
            p=pathlib.Path(d)
            (p/"target.txt").write_text("example")
            try: (p/"alias.txt").symlink_to(p/"target.txt")
            except OSError: self.skipTest("Symlink creation unavailable")
            with self.assertRaises(ValueError): ws.resolve(p,"alias.txt")
