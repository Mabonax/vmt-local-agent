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

    def test_repair_text_roles(self):
        theme = {'colour': {'background': '#FFFFFF', 'text': '#EEEEEE', 'primary': '#BBBBBB', 'accent': '#123456'}}
        changes = runner.repair_text_roles(theme)
        self.assertEqual(len(changes), 2)
        self.assertEqual(theme['colour']['accent'], '#123456')
        for role in ('text', 'primary'):
            self.assertGreaterEqual(runner.contrast_ratio(theme['colour'][role], '#FFFFFF'), 4.5)

    def test_checked_composition_blocks_failed_candidate(self):
        from unittest.mock import patch
        from docx import Document
        import fitz
        def word(data, output, theme):
            doc = Document()
            doc.add_paragraph('Native editable content')
            doc.save(output)
        def bad_render(*args):
            pdf = fitz.open()
            page = pdf.new_page()
            page.insert_text((50, 80), 'First line')
            page.insert_text((50, 80), 'Second line')
            pdf.save('bad.pdf')
            pdf.close()
            return 'bad.pdf'
        theme = {'colour': {'background': '#FFFFFF', 'text': '#111111', 'primary': '#111111'},
                 'quality': {'maximum_repair_attempts': 3}}
        with patch.object(runner, 'render_pdf', side_effect=bad_render):
            with self.assertRaisesRegex(ValueError, 'Structural review failed'):
                runner.compose_checked({'soffice': 'unused', 'args': {'output': 'out.docx'}}, {},
                                       {'word': word}, theme, Path('artifact.docx'), False)

    def test_type_scale(self):
        scale = runner.run({'args': {'operation': 'type_scale', 'mode': 'document', 'size': 11}})
        self.assertGreater(scale['title_pt'], scale['heading1_pt'])
        self.assertGreater(scale['heading1_pt'], scale['body_pt'])
        self.assertGreaterEqual(scale['caption_pt'], 9)
        with self.assertRaises(ValueError):
            runner.type_scale('invalid', 11)

    def test_measure_text_real_font(self):
        from unittest.mock import patch
        from PIL import ImageFont
        # Pillow ships a real embedded font; no OS font dependency for this fixture.
        with patch.object(ImageFont, 'truetype', wraps=ImageFont.truetype) as original:
            # Use an actual system font on Windows or Linux when available.
            font = 'Arial' if os.name == 'nt' else 'Liberation Sans'
            try:
                runner.reviewed_font(font)
            except (OSError, ValueError):
                self.skipTest('Reviewed system font unavailable')
            wide = runner.measure_text(font, 'A sentence with multiple words', 12, 400)
            narrow = runner.measure_text(font, 'A sentence with multiple words', 12, 40)
            self.assertGreater(narrow['line_count'], wide['line_count'])
            self.assertTrue(original.called)

    def test_layout_grid(self):
        Path('input').write_text(json.dumps({'width': 960, 'height': 540, 'margin': 30, 'gutter': 12, 'columns': 3, 'rows': 2}))
        grid = runner.run({'args': {'operation': 'layout_grid'}})
        self.assertEqual(len(grid['cells']), 6)
        self.assertAlmostEqual(grid['cells'][-1]['x'] + grid['cells'][-1]['w'], 930, places=3)
        with self.assertRaises(ValueError):
            runner.layout_grid({'width': 100, 'height': 100, 'margin': 50})

    def test_layout_check(self):
        data = {'width': 960, 'height': 540, 'margin': 20, 'elements': [
            {'id': 'a', 'x': 20, 'y': 20, 'w': 100, 'h': 50},
            {'id': 'b', 'x': 120, 'y': 20, 'w': 100, 'h': 50}]}
        self.assertEqual(runner.layout_check(data)['status'], 'PASS')
        data['elements'][1]['x'] = 10
        result = runner.layout_check(data)
        self.assertEqual(result['issue_count'], 2)
        self.assertEqual({i['type'] for i in result['issues']}, {'outside_safe_area', 'overlap'})
        data['elements'][1]['id'] = 'a'
        with self.assertRaises(ValueError):
            runner.layout_check(data)

    def test_arrange_equal_gaps_and_alignment(self):
        data = {'width':600,'height':300,'elements':[
            {'id':'a','x':10,'y':20,'w':30,'h':40},
            {'id':'b','x':80,'y':30,'w':40,'h':40},
            {'id':'c','x':210,'y':40,'w':50,'h':40}]}
        result = runner.layout_arrange({**data,'action':'distribute_x'})
        a,b,c = result['layout']['elements']
        self.assertAlmostEqual(b['x']-a['x']-a['w'],c['x']-b['x']-b['w'])
        aligned = runner.layout_arrange({**data,'action':'top'})
        self.assertEqual({e['y'] for e in aligned['layout']['elements']},{20})
        self.assertEqual(data['elements'][1]['y'],30)

    def test_svg_native_shapes_and_reject_markup(self):
        import xml.etree.ElementTree as ET
        data = {'width':600,'height':300,'elements':[
            {'id':'a','kind':'rect','x':10,'y':20,'w':100,'h':40,'fill':'#123456'},
            {'id':'b','kind':'ellipse','x':150,'y':20,'w':50,'h':50,'fill':'#FFFFFF'}]}
        Path('input').write_text(json.dumps(data))
        result = runner.run({'args':{'operation':'svg_compose','output':'out.svg'}})
        self.assertEqual(result['geometry_review']['status'],'PASS')
        root = ET.parse('artifact.svg').getroot()
        self.assertEqual(len(root),3)
        data['elements'][0]['onclick'] = 'alert(1)'
        with self.assertRaises(ValueError):
            runner.svg_compose(data,Path('bad.svg'))

    def test_svg_polygon_gradient_and_rejected_urls(self):
        import xml.etree.ElementTree as ET
        scene = {'width':600,'height':300,'elements':[{'id':'p','kind':'polygon','x':20,'y':20,'w':200,'h':100,
                 'points':[[0,0],[1,0],[0.5,1]],'fill':{'kind':'linear','direction':'diagonal','stops':[
                 {'offset':0,'colour':'#123456'},{'offset':1,'colour':'#FFFFFF'}]}}]}
        result = runner.svg_compose(scene,Path('gradient.svg'))
        self.assertEqual(result['status'],'PASS')
        root = ET.parse('gradient.svg').getroot()
        ns = {'s':'http://www.w3.org/2000/svg'}
        self.assertEqual(len(root.findall('.//s:stop',ns)),2)
        self.assertEqual(root.find('s:polygon',ns).get('fill'),'url(#gradient-p)')
        scene['elements'][0]['fill'] = 'url(https://example.invalid)'
        with self.assertRaises(ValueError):
            runner.svg_compose(scene,Path('bad.svg'))

    def test_svg_multiline_text(self):
        import xml.etree.ElementTree as ET
        from unittest.mock import patch
        from PIL import ImageFont
        face = ImageFont.load_default(size=48)
        scene = {'width':300,'height':300,'elements':[{'id':'t','kind':'text','x':10,'y':10,'w':90,'h':180,
                 'text':'First line of text\nSecond line','size':12,'fill':'#123456'}]}
        with patch.object(runner,'reviewed_font',return_value='unused'), patch.object(ImageFont,'truetype',return_value=face):
            runner.svg_compose(scene,Path('text.svg'))
            root = ET.parse('text.svg').getroot()
            lines = root.findall('.//{http://www.w3.org/2000/svg}tspan')
            self.assertGreaterEqual(len(lines),2)
            self.assertEqual(' '.join(n.text or '' for n in lines),'First line of text Second line')
            scene['elements'][0]['h'] = 1
            with self.assertRaises(ValueError):
                runner.svg_compose(scene,Path('overflow.svg'))

    def test_vector_png_pdf_export(self):
        import fitz
        from PIL import Image
        scene = {'width':300,'height':200,'background':'#FFFFFF','elements':[
            {'id':'red','kind':'rect','x':20,'y':20,'w':100,'h':100,'fill':'#FF0000'}]}
        Path('input').write_text(json.dumps(scene))
        for ext in ('png','pdf'):
            result = runner.run({'args':{'operation':'svg_compose','output':'out.'+ext}})
            self.assertEqual(result['format'],ext)
            self.assertEqual(result['visual_review'],'PENDING')
        with Image.open('artifact.png') as img:
            self.assertEqual(img.size,(600,400))
            self.assertEqual(img.getpixel((80,80))[:3],(255,0,0))
        with fitz.open('artifact.pdf') as doc:
            self.assertEqual(len(doc),1)
            self.assertAlmostEqual(doc[0].rect.width,300)
            self.assertAlmostEqual(doc[0].rect.height,200)
            self.assertTrue(doc[0].get_drawings())

    def test_image_format_allowlist(self):
        from PIL import Image, UnidentifiedImageError
        Image.new('RGB', (10, 10), 'red').save('input', format='TIFF')
        with self.assertRaises(UnidentifiedImageError):
            runner.run({'script': '', 'args': {'operation': 'extract'}})


if __name__ == '__main__':
    unittest.main()
