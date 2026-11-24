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

    def test_advanced_colour_dispatch(self):
        base = 'def distance(a,b): return 0.0\n'
        advanced = 'from colour import *\ndef system(a,b): return {"seed":a,"theme":b}\n'
        result = runner.run({'colour_base': base, 'script': advanced, 'args': {'operation': 'system', 'seed': '#123456', 'theme': 'dark'}})
        self.assertEqual(result, {'seed': '#123456', 'theme': 'dark'})
        with self.assertRaises(ValueError):
            runner.run({'colour_base': base, 'script': '', 'args': {'operation': 'system', 'seed': '#123456'}})

    def test_v2_design_system_and_dense_content(self):
        data = {'title': 'T', 'sections': [{'heading': 'H', 'paragraphs': ['x' * 300] * 6, 'table': [['a'], ['b']]}],
                'design_system': {'version': '2.0', 'colour': {'primary': '#123456'}}}
        runner.content(data, True, v2=True)
        runner.design_system(data, True)
        data['design_system']['typography']['font'] = '../unknown.ttf'
        with self.assertRaises(ValueError):
            runner.design_system(data, True)

    def test_structural_review_detects_overlap(self):
        import fitz
        pdf = fitz.open()
        page = pdf.new_page()
        page.insert_text((50, 80), 'First line')
        self.assertEqual(runner.structural_review(pdf)['structural_status'], 'PASS')
        page.insert_text((50, 80), 'Second line')
        result = runner.structural_review(pdf)
        self.assertEqual(result['structural_status'], 'REVIEW_REQUIRED')
        self.assertTrue(any(x['type'] == 'possible_text_overlap' for x in result['issues']))

    def test_image_format_allowlist(self):
        from PIL import Image, UnidentifiedImageError
        Image.new('RGB', (10, 10), 'red').save('input', format='TIFF')
        with self.assertRaises(UnidentifiedImageError):
            runner.run({'script': '', 'args': {'operation': 'extract'}})


if __name__ == '__main__':
    unittest.main()
