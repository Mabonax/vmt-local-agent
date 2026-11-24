import tempfile
import unittest
from artifact_store import ArtifactStore
from artifact_host import ArtifactHost

class ArtifactTests(unittest.TestCase):
    def test_pipeline(self):
        with tempfile.TemporaryDirectory() as d:
            store=ArtifactStore(d); host=ArtifactHost(store)
            a=host.call("pdf.generate",{"title":"Report","body":"Hello PDF"})["result"]
            self.assertNotIn("content",a)
            self.assertEqual(a["version"],1)
            b=host.call("pdf.generate",{"title":"Report","body":"Revision"},previous=a["artifact_id"])["result"]
            self.assertEqual(b["version"],2)
            self.assertEqual(b["previous"],a["artifact_id"])
            merged=host.call("pdf.merge",{"artifact_ids":[a["artifact_id"],b["artifact_id"]]})["result"]
            self.assertEqual(host.call("pdf.inspect",{"artifact_id":merged["artifact_id"]})["result"]["page_count"],2)
            self.assertIn("Hello PDF",host.call("pdf.extract_text",{"artifact_id":merged["artifact_id"],"pages":[1]})["result"]["pages"][0]["text"])
            self.assertEqual(host.call("artifact.hash",{"artifact_id":a["artifact_id"]})["result"]["sha256"],a["sha256"])
            self.assertEqual(len(host.call("artifact.list",{})["result"]["artifacts"]),3)
            self.assertEqual(host.call("artifact.metadata",{"artifact_id":a["artifact_id"]})["result"],a)
    def test_scope(self):
        with tempfile.TemporaryDirectory() as d:
            a=ArtifactStore(d); b=ArtifactStore(d)
            host=ArtifactHost(a)
            record=host.call("office.docx",{"title":"Report","sections":[]})["result"]
            with self.assertRaises(ValueError): b.retrieve(record["artifact_id"])
            with self.assertRaises(ValueError): a.retrieve("../outside")
            with self.assertRaises(ValueError): host.call("pdf.inspect",{"artifact_id":record["artifact_id"]})
            with self.assertRaises(ValueError): host.call("artifact.hash",{"content":"aGVsbG8="})
            self.assertEqual(host.call("artifact.inspect",{"artifact_id":record["artifact_id"]})["result"]["format"],"docx")
    def test_invalid_envelope(self):
        with tempfile.TemporaryDirectory() as d:
            store=ArtifactStore(d)
            with self.assertRaises(ValueError): store.store({"extension":"pdf","encoding":"base64","bytes":5,"content":"aGVsbG8="})
            self.assertEqual(store.list(),[])
    def test_tamper(self):
        with tempfile.TemporaryDirectory() as d:
            store=ArtifactStore(d)
            record=ArtifactHost(store).call("pdf.generate",{"title":"Report","body":"Example"})["result"]
            path=store._root/(record["artifact_id"]+".pdf")
            data=path.read_bytes()
            path.write_bytes(data[:-1]+bytes([data[-1]^1]))
            with self.assertRaises(ValueError): store.retrieve(record["artifact_id"])
