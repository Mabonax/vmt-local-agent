import hashlib
import unittest
from unittest.mock import patch
from evidence import EvidenceStore
from research_adapter import ResearchHost
from policy import PolicyHost

def fixture(text="The solar project generated electricity."):
    return {"url":"https://example.com/report","title":"Report","text":text,"sha256":hashlib.sha256(text.encode()).hexdigest(),"truncated":False,"links":[]}

class Tests(unittest.TestCase):
    def test_snapshot(self):
        store=EvidenceStore(); r=store.register_fetch(fixture())
        self.assertEqual(store.register_fetch(fixture())["source_id"],r["source_id"])
        self.assertNotIn("text",store.sources()[0])
        hits=store.find("solar electricity",3)
        self.assertEqual(hits[0]["source_id"],r["source_id"])
        self.assertEqual(hits[0]["excerpt"],fixture()["text"])
        self.assertEqual(store.citation(r["source_id"],4,9)["excerpt"],"solar")
        with self.assertRaises(ValueError): store.citation(r["source_id"],0,999)
        with self.assertRaises(ValueError): EvidenceStore().citation(r["source_id"],0,1)
    def test_unicode_offsets(self):
        store=EvidenceStore(); r=store.register_fetch(fixture("Straße solar"))
        self.assertEqual(store.find("strasse",1)[0]["excerpt"],"Straße solar")
        self.assertEqual(store.citation(r["source_id"],7,12)["excerpt"],"solar")
    def test_host(self):
        store=EvidenceStore()
        host=PolicyHost(ResearchHost(None,{"example.com"},evidence=store),{"web.fetch","research.sources","research.find","research.citation"},{"web.fetch"})
        args={"url":"https://example.com/report"}
        with patch("research_adapter.fetch",return_value=fixture()):
            r=host.call("web.fetch",args,approval=host.approve("web.fetch",args))["result"]
        sid=r["source_id"]
        self.assertEqual(host.call("research.sources",{})["result"]["sources"][0]["source_id"],sid)
        self.assertEqual(host.call("research.find",{"query":"solar","limit":1})["result"]["passages"][0]["source_id"],sid)
        self.assertEqual(host.call("research.citation",{"source_id":sid,"start":4,"end":9})["result"]["excerpt"],"solar")
        with self.assertRaises(PermissionError): host.call("research.source_add",fixture())
