import io,zipfile,unittest,hashlib
from lxml import etree as ET
from docx import Document
from document_cover import add_cover,NS

class CoverTests(unittest.TestCase):
    def fixture(self):
        doc=Document();doc.add_heading('Professional CV Template',0)
        doc.add_heading('Contact Information',1);doc.add_paragraph('[YOUR FULL NAME]')
        doc.add_paragraph('Email: [email@example.com] | Phone: [12345] | Location: [City]')
        doc.add_heading('Experience',1);doc.add_paragraph('Original experience must remain word for word.')
        doc.add_table(rows=1,cols=2).cell(0,0).text='Original table sentinel'
        doc.sections[0].header.paragraphs[0].text='Original header'
        doc.sections[0].footer.paragraphs[0].text='Original footer'
        data=io.BytesIO();doc.save(data);return data.getvalue()

    def test_cover_preserves_every_original_body_node_and_other_package_part(self):
        original=self.fixture();before=hashlib.sha256(original).hexdigest()
        result,proof=add_cover(original)
        self.assertEqual(hashlib.sha256(original).hexdigest(),before)
        self.assertTrue(proof['original_body_preserved']);self.assertTrue(proof['unchanged_package_parts'])
        with zipfile.ZipFile(io.BytesIO(original)) as source,zipfile.ZipFile(io.BytesIO(result)) as target:
            self.assertEqual(source.namelist(),target.namelist())
            for part in source.namelist():
                if part!='word/document.xml':self.assertEqual(source.read(part),target.read(part),part)
            old=ET.fromstring(source.read('word/document.xml')).find('w:body',NS)
            new=ET.fromstring(target.read('word/document.xml')).find('w:body',NS)
            self.assertEqual([ET.tostring(n) for n in old],[ET.tostring(n) for n in list(new)[-len(old):]])
        doc=Document(io.BytesIO(result));self.assertEqual(doc.paragraphs[0].text,'Curriculum Vitae')
        self.assertIn('[YOUR FULL NAME]',doc.paragraphs[1].text)
        self.assertEqual(doc.tables[-1].cell(0,0).text,'Original table sentinel')
        self.assertEqual(doc.tables[0].cell(0,0).text,'Email')
        self.assertEqual(len(doc.tables),2)
        with zipfile.ZipFile(io.BytesIO(result)) as package:
            self.assertIn(b'fillcolor="#172B45"',package.read('word/document.xml'))
        self.assertEqual(doc.sections[0].header.paragraphs[0].text,'Original header')

    def test_empty_and_signed_documents_are_rejected(self):
        empty=io.BytesIO();Document().save(empty)
        with self.assertRaises(ValueError):add_cover(empty.getvalue())
        signed=io.BytesIO(self.fixture())
        with zipfile.ZipFile(signed,'a') as archive:archive.writestr('_xmlsignatures/sig1.xml','signed')
        with self.assertRaises(ValueError):add_cover(signed.getvalue())

if __name__=='__main__':unittest.main()
