import tempfile
import unittest
from system_adapter import SystemHost,estimate
from artifact_host import ArtifactHost
from artifact_store import ArtifactStore
from policy import PolicyHost

class Tests(unittest.TestCase):
    def test_estimate(self):
        a={"parameters_million":7000,"bits_per_weight":4,"context_tokens":4096,"kv_bytes_per_token":65536,"runtime_overhead_mib":512,"available_mib":6144}
        r=estimate(a)
        self.assertEqual(r["weights_mib"],3338)
        self.assertEqual(r["kv_cache_mib"],256)
        self.assertTrue(r["fits_estimate"])
        with self.assertRaises(ValueError): estimate({**a,"bits_per_weight":True})
    def test_policy(self):
        with tempfile.TemporaryDirectory() as d:
            host=PolicyHost(SystemHost(ArtifactHost(ArtifactStore(d)),d),{"system.info","system.storage","system.memory","system.cpu","system.dependencies"})
            for name in ("system.info","system.storage","system.memory","system.cpu","system.dependencies"):
                self.assertTrue(host.call(name,{})["ok"])
            self.assertNotIn("hostname",host.call("system.info",{})["result"])
            self.assertNotIn("path",host.call("system.storage",{})["result"])
            with self.assertRaises(ValueError): host.call("system.storage",{"path":"/"})
            with self.assertRaises(PermissionError): host.call("system.processes",{})
