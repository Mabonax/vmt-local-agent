import unittest
from runner import execute
class Tests(unittest.TestCase):
    def test_roundtrip(self):
        a=execute("pdf.generate",{"title":"Report","body":"Hello PDF"})
        self.assertEqual(execute("pdf.inspect",{"content":a["content"]})["page_count"],1)
        self.assertIn("Hello PDF",execute("pdf.extract_text",{"content":a["content"],"pages":[1]})["pages"][0]["text"])
        b=execute("pdf.merge",{"documents":[a["content"],a["content"]]})
        self.assertEqual(execute("pdf.inspect",{"content":b["content"]})["page_count"],2)
        c=execute("pdf.split",{"content":b["content"],"pages":[2]})
        self.assertEqual(execute("pdf.inspect",{"content":c["content"]})["page_count"],1)
        with self.assertRaises(ValueError): execute("pdf.split",{"content":a["content"],"pages":[0]})
    def test_bad_input(self):
        with self.assertRaises(ValueError): execute("pdf.inspect",{"content":"aGVsbG8="})
