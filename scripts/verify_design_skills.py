"""Explicit local acceptance through the desktop's real Rust execution broker.

Run using the isolated design Python. Creates only synthetic fixtures in --output.
Original skill archives and user projects are never modified.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile
from PIL import Image
from docx import Document
from pptx import Presentation


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--agent', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    root = args.output.resolve()
    agent = args.agent.resolve()
    receipt = {'operations': [], 'scope': 'synthetic fixtures; explicit operator CLI approvals', 'visual_review': 'PENDING'}

    def call(**request):
        result = subprocess.run([str(agent), '--design-operation', json.dumps(request), str(root)], capture_output=True, text=True, encoding='utf-8', timeout=130)
        if result.returncode:
            raise RuntimeError(result.stderr + result.stdout)
        value = json.loads(result.stdout)
        receipt['operations'].append(request['operation'])
        return value

    contrast = call(operation='contrast', foreground='#000000', background='#FFFFFF')
    assert contrast['ratio'] == 21 and contrast['AA_normal']
    for theme in ['light', 'dark']:
        palette = call(operation='palette', seed='#087F83', mode='analogous', theme=theme)
        assert palette['roles']['brand_seed'] == '#087F83'
        assert all(check['AA_normal'] for check in palette['checks'])
        (root / f'palette-{theme}.json').write_text(json.dumps(palette, indent=2))
    image = Image.new('RGB', (120, 80), 'red'); image.paste('blue', (60, 0, 120, 80)); image.save(root / 'colours.png')
    extracted = call(operation='extract', input='colours.png', count=2)
    extracted = extracted.get('result', extracted) if isinstance(extracted, dict) else extracted
    assert {item['hex'] for item in extracted} == {'#FF0000', '#0000FF'}
    data = {'title': 'Local design acceptance', 'subtitle': 'Editable outputs from the reviewed skill adapter', 'sections': [{'heading': 'Purpose and scope', 'paragraphs': ['Verify the local agent can create editable documents and presentations.'], 'bullets': ['Preserve the supplied source text', 'Render every page and slide']}, {'heading': 'Next steps', 'paragraphs': ['Inspect the rendered output before delivery.'], 'bullets': ['Check typography and spacing', 'Record the review result']}]}
    (root / 'content.json').write_text(json.dumps(data), encoding='utf-8')
    word = json.loads(json.dumps(data)); word['sections'][1]['table'] = [['Check', 'Required result'], ['Source content', 'Complete'], ['Visual review', 'Required']]
    (root / 'word-content.json').write_text(json.dumps(word), encoding='utf-8')
    for input_name, output in [('word-content.json', 'acceptance.docx'), ('content.json', 'acceptance.pptx')]:
        result = call(operation='compose', input=input_name, output=output)
        assert result['visual_review'] == 'PENDING'
        assert hashlib.sha256((root / output).read_bytes()).hexdigest() == result['sha256']
        # The same operation cannot overwrite its previous output.
        blocked = subprocess.run([str(agent), '--design-operation', json.dumps({'operation':'compose','input':input_name,'output':output}), str(root)], capture_output=True)
        assert blocked.returncode != 0
        assert hashlib.sha256((root / output).read_bytes()).hexdigest() == result['sha256']
        review = call(operation='render', input=output, output='review-' + Path(output).suffix[1:])
        assert review['render_status'] == 'PASS' and review['visual_review'] == 'PENDING'
        assert review['pages'] == (1 if output.endswith('.docx') else 3)
        receipt[output] = {'sha256': result['sha256'], 'pages': review['pages']}
    document = Document(root / 'acceptance.docx')
    text = '\n'.join(p.text for p in document.paragraphs)
    assert data['title'] in text
    assert all(s['heading'] in text and all(v in text for v in s.get('paragraphs', []) + s.get('bullets', [])) for s in data['sections'])
    assert document.tables[0].rows[1].cells[1].text == 'Complete'
    slides = Presentation(root / 'acceptance.pptx')
    all_text = '\n'.join(shape.text for slide in slides.slides for shape in slide.shapes if shape.has_text_frame)
    assert '\ufffd' not in all_text
    assert all(s['heading'] in all_text and all(v in all_text for v in s.get('paragraphs', []) + s.get('bullets', [])) for s in data['sections'])
    assert len(slides.slides) == 3
    for name in ['acceptance.docx', 'acceptance.pptx']:
        with zipfile.ZipFile(root / name) as archive:
            assert any('document.xml' in part or 'slide1.xml' in part for part in archive.namelist())
    receipt['passed'] = True
    (root / 'acceptance.json').write_text(json.dumps(receipt, indent=2), encoding='utf-8')
    print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
