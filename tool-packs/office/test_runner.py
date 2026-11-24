import base64
import io
import unittest
from runner import execute
from docx import Document
from openpyxl import load_workbook
from pptx import Presentation

def artifact(name,args):
    r=execute(name,args)
    data=base64.b64decode(r["content"],validate=True)
    assert len(data)==r["bytes"]
    return io.BytesIO(data)

class Tests(unittest.TestCase):
    def test_docx(self):
        d=Document(artifact("office.docx",{"title":"Report","sections":[{"heading":"Summary","body":"Test paragraph"}]}))
        self.assertEqual([p.text for p in d.paragraphs],["Report","Summary","Test paragraph"])
    def test_xlsx(self):
        w=load_workbook(artifact("office.xlsx",{"headers":["Name","Value"],"rows":[["=1+1",42]]}))
        self.assertEqual(w.active["A2"].data_type,"s")
        self.assertEqual(w.active["B2"].value,42)
        self.assertEqual(w.active.freeze_panes,"A2")
    def test_pptx(self):
        p=Presentation(artifact("office.pptx",{"title":"Deck","slides":[{"heading":"Summary","body":"A point"}]}))
        self.assertEqual(len(p.slides),2)
        self.assertEqual(p.slides[1].shapes[0].text,"Summary")
    def test_reject(self):
        with self.assertRaises(ValueError): execute("office.xlsx",{"headers":["A"],"rows":[[1,2]]})
