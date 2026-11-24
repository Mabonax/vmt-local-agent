import tempfile
import threading
import unittest
from artifact_store import ArtifactStore
from artifact_host import ArtifactHost
from workflow import run
from workflow_templates import get_template

class Tests(unittest.TestCase):
    def test_pdf_pipeline(self):
        with tempfile.TemporaryDirectory() as d:
            host=ArtifactHost(ArtifactStore(d))
            report=run(host,get_template("pdf_preview"),{"title":"Report","body":"Example"})
            self.assertEqual(report["status"],"passed")
            self.assertEqual(len(report["journal"]),5)
            self.assertEqual(report["outputs"]["preview"]["extension"],"png")
            self.assertNotIn("content",report["outputs"]["document"])
    def test_gate_failure(self):
        class Host:
            def call(self,name,args): return {"ok":True,"result":{"value":0}}
        plan={"steps":[{"id":"a","tool":"data.describe","args":{},"gates":[{"value":{"ref":"step.a.value"},"op":"gt","target":1}]},{"id":"b","tool":"data.describe","args":{}}],"outputs":{}}
        r=run(Host(),plan,{})
        self.assertEqual(r["status"],"failed")
        self.assertEqual(len(r["journal"]),1)
        self.assertEqual(r["journal"][0]["status"],"gate_failed")
    def test_cancel(self):
        event=threading.Event(); event.set()
        r=run(None,get_template("pdf_preview"),{},cancel=event)
        self.assertEqual(r["status"],"cancelled")
        self.assertEqual(r["journal"],[])
    def test_reject_tool(self):
        with self.assertRaises(ValueError): run(None,{"steps":[{"id":"x","tool":"shell.run","args":{}}],"outputs":{}},{})
    def test_missing_reference(self):
        r=run(None,get_template("pdf_preview"),{})
        self.assertEqual(r["status"],"failed")
        self.assertEqual(r["journal"][0]["status"],"error")
