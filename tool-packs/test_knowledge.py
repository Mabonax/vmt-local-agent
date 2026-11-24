import tempfile
import unittest
from knowledge import KnowledgeStore,KnowledgeHost
from policy import PolicyHost

class Tests(unittest.TestCase):
    def test_persistence_and_revision(self):
        with tempfile.TemporaryDirectory() as d:
            s=KnowledgeStore(d)
            try:
                a=s.ingest("policy","Policy","Solar energy policy")
                self.assertEqual(s.ingest("policy","Policy","Solar energy policy"),a)
                b=s.ingest("policy","Policy","Wind energy policy")
                self.assertEqual(b["revision"],2)
                self.assertEqual(s.search("solar",5),[])
                self.assertEqual(s.search("wind",5)[0]["document_id"],b["document_id"])
                self.assertEqual(s.citation(a["document_id"],0,5)["excerpt"],"Solar")
            finally: s.close()
            s=KnowledgeStore(d)
            try:
                self.assertEqual(s.statistics()["revisions"],2)
                self.assertEqual(s.search("wind",5)[0]["document_id"],b["document_id"])
                self.assertEqual(s.search('" OR *',5),[])
            finally: s.close()
    def test_policy_and_collection_scope(self):
        with tempfile.TemporaryDirectory() as a,tempfile.TemporaryDirectory() as b:
            s=KnowledgeStore(a); t=KnowledgeStore(b)
            try:
                doc=s.ingest("manual","Manual","Unicode Straße and solar")
                host=PolicyHost(KnowledgeHost(None,s),{"knowledge.search","knowledge.citation","knowledge.statistics","knowledge.documents"})
                self.assertEqual(host.call("knowledge.search",{"query":"solar","limit":1})["result"]["passages"][0]["document_id"],doc["document_id"])
                with self.assertRaises(ValueError): t.citation(doc["document_id"],0,1)
                with self.assertRaises(PermissionError): host.call("knowledge.ingest",{})
                with self.assertRaises(ValueError): host.call("knowledge.citation",{"document_id":doc["document_id"],"start":0,"end":999})
            finally: s.close(); t.close()
