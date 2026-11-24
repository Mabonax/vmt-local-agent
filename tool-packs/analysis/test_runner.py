import unittest
from runner import execute
class Tests(unittest.TestCase):
    def test_hash(self):
        self.assertEqual(execute("text.sha256",{"text":""})["sha256"],"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    def test_diff(self):
        self.assertIn("+b",execute("text.diff",{"before":"a","after":"b"})["lines"])
    def test_csv(self):
        self.assertEqual(execute("csv.profile",{"text":"a,b\n1,\n"})["columns"][1]["missing"],1)
        with self.assertRaises(ValueError): execute("csv.profile",{"text":"a,b\n1"})
    def test_symbols(self):
        self.assertEqual(execute("python.symbols",{"text":"def f(): pass"})["symbols"][0]["name"],"f")
    def test_find(self):
        self.assertEqual(execute("text.find",{"text":"A\nb","query":"a"})["matches"][0]["line"],1)
    def test_reject(self):
        with self.assertRaises(ValueError): execute("text.find",{"text":"a","query":""})
