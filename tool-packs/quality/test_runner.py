import base64
import io
import unittest
from runner import execute
from docx import Document
class Tests(unittest.TestCase):
    def test_hash(self):
        self.assertEqual(execute("artifact.hash",{"content":"aGVsbG8="})["bytes"],5)
    def test_docx(self):
        b=io.BytesIO(); Document().save(b)
        self.assertEqual(execute("artifact.inspect",{"content":base64.b64encode(b.getvalue()).decode()})["format"],"docx")
    def test_invalid(self):
        with self.assertRaises(Exception): execute("artifact.inspect",{"content":"aGVsbG8="})
    def test_geometry(self):
        args={"width":100,"height":100,"elements":[{"id":"a","x":0,"y":0,"width":20,"height":20},{"id":"b","x":10,"y":10,"width":100,"height":20}]}
        self.assertEqual(execute("layout.bounds",args)["outside"],["b"])
        self.assertEqual(execute("layout.overlap",args)["overlaps"][0]["area"],100)
    def test_touching(self):
        a={"width":100,"height":100,"elements":[{"id":"a","x":0,"y":0,"width":10,"height":10},{"id":"b","x":10,"y":0,"width":10,"height":10}]}
        self.assertEqual(execute("layout.overlap",a)["count"],0)
