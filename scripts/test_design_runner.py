"""Dependency-enabled adapter tests. Run with the isolated design Python runtime."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('design_runner', Path(__file__).with_name('design_runner.py'))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class DesignRunnerTests(unittest.TestCase):
    def setUp(self):
        self.fixture = tempfile.TemporaryDirectory()
        self.previous = Path.cwd()
        os.chdir(self.fixture.name)

    def tearDown(self):
        os.chdir(self.previous)
        self.fixture.cleanup()

    def test_rejects_document_fields_and_code_as_data(self):
        for data in [{'title': 'X', 'sections': [], 'script': 'bad'}, {'title': 'X', 'sections': []}, {'title': '\x00', 'sections': [{}]}]:
            with self.assertRaises((ValueError, KeyError)):
                runner.content(data, False)

    def test_rejects_dense_slides_and_unsupported_tables(self):
        for section in [{'heading': 'H', 'paragraphs': ['X'] * 6}, {'heading': 'H', 'table': [['a']]}, {'heading': 'H' * 76}]:
            with self.assertRaises(ValueError):
                runner.content({'title': 'Title', 'sections': [section]}, True)

    def test_word_table_shape_and_text_limits(self):
        with self.assertRaises(ValueError):
            runner.content({'title': 'T', 'sections': [{'heading': 'H', 'table': [['a', 'b'], ['c']]}]}, False)
        with self.assertRaises(ValueError):
            runner.content({'title': 'T', 'sections': [{'heading': 'H', 'paragraphs': ['x' * 4001]}]}, False)

    def test_rejects_external_relationship_and_active_objects(self):
        for name, data in [('word/_rels/document.xml.rels', '<Relationships><Relationship TargetMode="External" Target="https://example.invalid"/></Relationships>'), ('word/vbaProject.bin', 'anything'), ('word/embeddings/object.bin', 'anything'), ('../outside.xml', '<root/>'), ('word/document.xml', '<!DOCTYPE a [<!ENTITY x "bad">]><a/>'), ('word/document.xml', '<x xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:instrText>INCLUDETEXT secret</w:instrText></x>')]:
            with zipfile.ZipFile('unsafe.docx', 'w') as archive:
                archive.writestr(name, data)
            with self.assertRaises(ValueError):
                runner.validate_office(Path('unsafe.docx'))

    def test_preserves_text_and_native_editability(self):
        from docx import Document
        from pptx import Presentation
        from docx.shared import Mm
        # Exercise our validator with the actual dependency APIs, no external script.
        data = {'title': 'Test overview', 'sections': [{'heading': 'Scope', 'paragraphs': ['Exact source text.'], 'bullets': ['Verify outputs']}]}
        runner.content(data, False)
        document = Document(); document.sections[0].page_width = Mm(210)
        document.add_heading(data['title'], 0); document.add_paragraph(data['sections'][0]['paragraphs'][0]); document.save('test.docx')
        runner.validate_office(Path('test.docx'))
        self.assertIn('Exact source text.', '\n'.join(p.text for p in Document('test.docx').paragraphs))
        presentation = Presentation(); slide = presentation.slides.add_slide(presentation.slide_layouts[6]); slide.shapes.add_textbox(0, 0, 1000000, 1000000).text = 'Editable text'; presentation.save('test.pptx')
        runner.validate_office(Path('test.pptx'))
        self.assertEqual(Presentation('test.pptx').slides[0].shapes[0].text, 'Editable text')

    def test_image_format_allowlist(self):
        from PIL import Image, UnidentifiedImageError
        Image.new('RGB', (10, 10), 'red').save('input', format='TIFF')
        with self.assertRaises(UnidentifiedImageError):
            runner.run({'script': '', 'args': {'operation': 'extract'}})

    def test_existing_cv_formatting_preserves_all_text_and_opaque_parts(self):
        from docx import Document
        from lxml import etree
        document=Document()
        document.add_paragraph('CV template', 'Title')
        document.add_heading('Experience',1)
        document.add_paragraph('[Job Title] | [Company] | [Year]')
        document.add_paragraph('- Preserve every placeholder and word')
        document.add_table(rows=1,cols=1).cell(0,0).text='Table source'
        document.sections[0].header.paragraphs[0].text='Preserved header'
        document.save('input')
        inspected=runner.run({'args':{'operation':'inspect_document','input':'cv.docx'}})
        self.assertEqual(inspected['paragraph_count'],4)
        self.assertEqual(inspected['tables'],[[['Table source']]])
        result=runner.run({'args':{'operation':'format_document','input':'cv.docx','output':'new.docx','preset':'cv','accent':'#007B7F'}})
        self.assertTrue(result['content_preserved'])
        self.assertNotEqual(result['sha256'],result['source_sha256'])
        self.assertEqual(result['visual_review'],'PENDING')
        with zipfile.ZipFile('input') as before,zipfile.ZipFile('artifact.docx') as after:
            self.assertEqual(before.namelist(),after.namelist())
            for name in before.namelist():
                if name not in ['word/document.xml','word/styles.xml']:
                    self.assertEqual(before.read(name),after.read(name))
            ns={'w':'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
            original=etree.fromstring(before.read('word/document.xml'))
            formatted=etree.fromstring(after.read('word/document.xml'))
            self.assertEqual(original.xpath('//w:t/text()',namespaces=ns),formatted.xpath('//w:t/text()',namespaces=ns))
            self.assertIn('007B7F',after.read('word/document.xml').decode())

    def test_existing_cv_rejects_unreadable_accent(self):
        from docx import Document
        Document().save('input')
        with self.assertRaisesRegex(ValueError,'4.5:1'):
            runner.run({'args':{'operation':'format_document','input':'cv.docx','output':'new.docx','accent':'#FFFFFF'}})
        self.assertFalse(Path('artifact.docx').exists())


if __name__ == '__main__':
    unittest.main()
