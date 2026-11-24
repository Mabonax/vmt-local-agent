import pathlib
import subprocess
import sys
import tempfile
import threading
import unittest
from artifact_host import ArtifactHost
from artifact_store import ArtifactStore
from policy import PolicyHost
from execution import run_bounded
from integrity import snapshot,verify

class Tests(unittest.TestCase):
    def test_policy(self):
        with tempfile.TemporaryDirectory() as d:
            host=PolicyHost(ArtifactHost(ArtifactStore(d)),{"math.calculate"},{"math.calculate"})
            args={"operation":"add","a":2,"b":3}
            with self.assertRaises(PermissionError): host.call("math.calculate",args)
            token=host.approve("math.calculate",args)
            self.assertEqual(host.call("math.calculate",args,approval=token)["result"]["value"],5)
            with self.assertRaises(PermissionError): host.call("math.calculate",args,approval=token)
            token=host.approve("math.calculate",args)
            with self.assertRaises(PermissionError): host.call("math.calculate",{**args,"a":9},approval=token)
            with self.assertRaises(PermissionError): host.call("pdf.generate",{})
            self.assertEqual([x["status"] for x in host.journal()],["denied","passed","denied","denied","denied"])
            self.assertNotIn("args",host.journal()[1])
    def test_integrity(self):
        baseline=snapshot()
        verify(baseline)
        with self.assertRaises(PermissionError): verify({})
    def test_output_cap(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                run_bounded([sys.executable,"-I","-c","print('x'*100000)"],b"",d,5,1000)
    def test_timeout(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(subprocess.TimeoutExpired):
                run_bounded([sys.executable,"-I","-c","import time; time.sleep(5)"],b"",d,.2,1000)
    def test_active_cancel(self):
        event=threading.Event()
        timer=threading.Timer(.2,event.set); timer.start()
        try:
            with tempfile.TemporaryDirectory() as d:
                with self.assertRaises(ValueError):
                    run_bounded([sys.executable,"-I","-c","import time; time.sleep(5)"],b"",d,5,1000,event)
        finally: timer.cancel(); timer.join()
