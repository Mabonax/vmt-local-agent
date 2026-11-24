"""Operator-invoked synthetic acceptance through the real Rust review broker.

Use the configured private Python. Each request below is a fixed fixture, never
model-generated approval. Outputs must be new and outside the source tree.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
import urllib.request

from PIL import Image, ImageDraw
from docx import Document
from pptx import Presentation


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--agent', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--profile', type=Path)
    parser.add_argument('--vision-model', help='Explicit installed local model; no downloads')
    args = parser.parse_args()
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=False)
    agent = str(args.agent.resolve())
    report = {'operations': [], 'documents': [], 'graphics': [], 'failures': [],
              'scope': 'Explicit operator fixture approvals through the Rust broker',
              'visual_review': 'PENDING'}

    def save(name, value):
        (root / name).write_text(json.dumps(value, indent=2), encoding='utf-8')

    def call(operation, **kwargs):
        request = {'operation': operation, **kwargs}
        command = [agent, '--design-operation', json.dumps(request), str(root)]
        if args.profile:
            command += ['--data-dir', str(args.profile.resolve())]
        started = time.monotonic()
        result = subprocess.run(command, capture_output=True, encoding='utf-8', timeout=135)
        row = {'request': request, 'seconds': round(time.monotonic() - started, 3),
               'exit_code': result.returncode}
        try:
            if result.returncode:
                raise RuntimeError(result.stderr.strip() or result.stdout.strip())
            value = json.loads(result.stdout)
            row['result'] = value
            if operation == 'vision_inspect':
                try:
                    # Observe actual unload state; the worker only requests it.
                    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
                    with opener.open('http://127.0.0.1:11434/api/ps', timeout=5) as response:
                        row['loaded_models_after'] = [m['name'] for m in json.load(response)['models']]
                except Exception as exc:
                    row['unload_measurement_error'] = type(exc).__name__
            if kwargs.get('output') and (root / kwargs['output']).is_file():
                assert hashlib.sha256((root / kwargs['output']).read_bytes()).hexdigest() == value['sha256']
            assert 'broker_provenance' in value
            return value
        except Exception as exc:
            row['error'] = str(exc)
            report['failures'].append(row)
            return None
        finally:
            report['operations'].append(row)
            save('acceptance.json', report)
            print(operation, kwargs.get('output', kwargs.get('mode', '')), result.returncode, flush=True)

    # Entirely synthetic landscape and portrait fixtures, no personal assets.
    photo = Image.new('RGB', (960, 640), '#9ac8dd')
    draw = ImageDraw.Draw(photo)
    draw.ellipse((660, 55, 800, 195), fill='#ffcf79')
    draw.polygon([(0, 440), (250, 140), (490, 440)], fill='#445a70')
    draw.polygon([(300, 500), (640, 220), (960, 500)], fill='#637f68')
    draw.rectangle((0, 480, 960, 640), fill='#256c79')
    photo.save(root / 'original-landscape.jpg', quality=94)
    portrait = Image.new('RGB', (400, 500), '#d5e7eb')
    d = ImageDraw.Draw(portrait)
    d.ellipse((105, 60, 295, 250), fill='#ad7759')
    d.ellipse((45, 270, 355, 620), fill='#172b45')
    portrait.save(root / 'synthetic-portrait.png')
    logo = Image.new('RGBA', (240, 240))
    ImageDraw.Draw(logo).polygon([(120, 20), (220, 200), (20, 200)], fill='#087f83')
    logo.save(root / 'transparent-logo.png')

    for op, kw in [
        ('palette', dict(seed='#087F83', mode='analogous', theme='light')),
        ('contrast', dict(foreground='#000000', background='#FFFFFF')),
        ('extract', dict(input='original-landscape.jpg', count=4)),
        ('repair', dict(foreground='#EEEEEE', background='#FFFFFF')),
        ('gradient', dict(foreground='#172B45', background='#087F83', count=7)),
        ('categorical', dict(seed='#087F83', count=5)),
        ('simulate', dict(seed='#087F83', mode='deutan')),
        ('system', dict(seed='#087F83', theme='light')),
        ('distance', dict(foreground='#000000', background='#FFFFFF')),
        ('type_scale', dict(mode='document', size=11)),
        ('measure_text', dict(font='Arial', sample='Local creative engine acceptance', size=18, width=300)),
        ('image_inspect', dict(input='original-landscape.jpg')),
    ]:
        call(op, **kw)

    layout = {'width': 600, 'height': 400, 'margin': 20, 'elements': [
        {'id': 'a', 'x': 20, 'y': 20, 'w': 100, 'h': 60},
        {'id': 'b', 'x': 200, 'y': 30, 'w': 100, 'h': 60},
        {'id': 'c', 'x': 440, 'y': 40, 'w': 100, 'h': 60}]}
    save('grid.json', {'width': 600, 'height': 400, 'margin': 20, 'gutter': 12, 'columns': 3, 'rows': 2})
    save('layout.json', layout)
    save('arrange.json', {**layout, 'action': 'distribute_x'})
    for op, source in [('layout_grid', 'grid.json'), ('layout_check', 'layout.json'), ('layout_arrange', 'arrange.json')]:
        call(op, input=source)
    for mode, output in [('contain', 'resized.png'), ('cover', 'cropped.png')]:
        call('image_prepare', input='original-landscape.jpg', output=output, width=480, height=480, mode=mode)
    for mode, amount in [('brightness', 110), ('contrast', 115), ('saturation', 120), ('sharpness', 140), ('grayscale', None), ('rounded', 20), ('circle', None)]:
        source = 'synthetic-portrait.png' if mode == 'circle' else 'original-landscape.jpg'
        call('image_adjust', input=source, output=mode + '.png', mode=mode, **({'amount': amount} if amount is not None else {}))
    call('image_adjust', input='transparent-logo.png', output='logo-adjusted.png', mode='contrast', amount=120)

    sections = [
        {'heading': 'Purpose and recommendation', 'paragraphs': ['This synthetic proposal recommends a four week pilot of local creative tools. The pilot preserves editable source files and requires review before delivery.']},
        {'heading': 'Delivery approach', 'paragraphs': ['Define the brief, create an editable candidate, render every page and record the review.'], 'bullets': ['Preserve supplied wording', 'Review each generated artifact', 'Record decisions and limitations']},
        {'heading': 'Milestones', 'table': [['Week', 'Deliverable', 'Evidence'], ['1', 'Requirements', 'Approved brief'], ['2', 'Prototype', 'Editable candidate'], ['3', 'Validation', 'Review images'], ['4', 'Handover', 'Acceptance record']]},
    ]
    long_paragraphs = [f'Observation {i:02}. The synthetic team reviewed local document generation and retained all source wording. Each review records typography, margins, table structure and follow up actions. The output remains subject to a human decision before operational use.' for i in range(1, 33)]
    documents = [
        ('professional-cv.docx', {'title': 'Alex Example', 'subtitle': 'Synthetic technical specialist CV', 'sections': [
            {'heading': 'Professional profile', 'paragraphs': ['Technical specialist with synthetic experience in software delivery, digital fabrication and training. This fixture contains no real employment claims.']},
            {'heading': 'Selected experience', 'bullets': ['Delivered an internal software prototype', 'Coordinated practical design workshops', 'Documented testing and handover requirements']},
            {'heading': 'Education and skills', 'table': [['Area', 'Capability'], ['Software', 'Rust and Python'], ['Design', 'Editable documents and vector graphics']]}]}),
        ('business-proposal.docx', {'title': 'Local creative tools pilot proposal', 'subtitle': 'Synthetic business proposal', 'sections': sections}),
        ('multi-page-report.docx', {'title': 'Local creative tools evaluation', 'sections': [{'heading': 'Evaluation findings', 'paragraphs': long_paragraphs[:16]}, sections[2]]}),
        ('native-table-report.docx', {'title': 'Pilot delivery milestones', 'sections': [sections[0], sections[2]]}),
        ('long-content-report.docx', {'title': 'Extended local evaluation report', 'sections': [{'heading': 'Detailed observations', 'paragraphs': long_paragraphs}]}),
        ('business-presentation.pptx', {'title': 'Local creative tools pilot', 'subtitle': 'Synthetic business presentation', 'sections': sections[:2]}),
        ('ten-slide-presentation.pptx', {'title': 'Pilot delivery review', 'sections': [{'heading': f'Review topic {i}', 'bullets': ['Confirm the objective', 'Review the evidence', 'Record the next action']} for i in range(1, 10)]}),
        ('native-table-presentation.pptx', {'title': 'Delivery milestones', 'sections': [sections[2]]}),
        ('long-content-presentation.pptx', {'title': 'Extended evaluation findings', 'sections': [{'heading': 'Detailed observations', 'paragraphs': long_paragraphs[:8]}]}),
    ]
    for output, content in documents:
        content['design_system'] = {'typography': {'font': 'Arial'}}
        source = output + '.json'
        save(source, content)
        result = call('compose_v2', input=source, output=output)
        if not result:
            continue
        review = call('render', input=output, output='review-' + Path(output).stem)
        if output.endswith('.docx'):
            doc = Document(root / output)
            actual = '\n'.join([p.text for p in doc.paragraphs] + [c.text for t in doc.tables for r in t.rows for c in r.cells])
            tables = len(doc.tables)
        else:
            deck = Presentation(root / output)
            actual = '\n'.join(s.text if s.has_text_frame else '\n'.join(c.text for r in s.table.rows for c in r.cells) if s.has_table else '' for p in deck.slides for s in p.shapes)
            headings = {s['heading'] + suffix for s in content['sections'] for suffix in ['', ' — continued', ' — table']}
            body = '\n'.join(s.text for page in deck.slides for s in page.shapes if s.has_text_frame and s.text.strip() not in headings and not s.text.strip().isdigit())
            tables = sum(s.has_table for p in deck.slides for s in p.shapes)
        expected = [content['title']] + [v for s in content['sections'] for v in s.get('paragraphs', []) + s.get('bullets', []) + [c for r in s.get('table', []) for c in r]]
        normalized = ' '.join(actual.split())
        body_normalized = ' '.join(body.split()) if output.endswith('.pptx') else normalized
        preserved = all(' '.join(v.split()) in normalized or ' '.join(v.split()) in body_normalized for v in expected)
        row = {'path': output, 'review': review, 'native_tables': tables, 'content_preserved': preserved}
        report['documents'].append(row)
        if not preserved:
            report['failures'].append({'content_missing': output})
        if output == 'ten-slide-presentation.pptx' and len(deck.slides) != 10:
            report['failures'].append({'ten_slide_count': len(deck.slides)})

    # V1 composition and existing-document operations remain exercised as well.
    save('baseline.json', {'title': 'Baseline contract', 'sections': [sections[0]]})
    call('compose', input='baseline.json', output='baseline.docx')
    call('render', input='baseline.docx', output='review-baseline')
    call('inspect_document', input='professional-cv.docx')
    call('format_document', input='professional-cv.docx', output='formatted-cv.docx', preset='cv', accent='#087F83')
    call('render', input='formatted-cv.docx', output='review-formatted-cv')

    from copy import deepcopy
    for name, template in [('event-poster', 'poster'), ('business-card', 'business_card'), ('social-graphic', 'social'), ('certificate', 'certificate')]:
        brief = {'template': template, 'title': 'Digital Innovation Day' if template == 'poster' else 'Example Studio' if template != 'certificate' else 'Certificate of Achievement',
                 'subtitle': 'Creative tools and practical ideas', 'footer': 'Synthetic acceptance fixture'}
        if template != 'certificate':
            brief['asset'] = 0
        save(name + '-brief.json', brief)
        call('template_plan', input=name + '-brief.json')
        plan = call('creative_plan', input=name + '-brief.json')
        if not plan:
            continue
        scene = plan['candidates'][0]['scene']
        save(name + '.json', scene)
        call('scene_check', input=name + '.json')
        for ext in ['svg', 'png', 'pdf']:
            call('svg_compose', input=name + '.json', output=name + '.' + ext, **({'assets': ['original-landscape.jpg']} if template != 'certificate' else {}))
        report['graphics'].append(name)

    scene = {'width': 600, 'height': 400, 'background': '#FFFFFF', 'elements': [
        {'id': 'band', 'kind': 'rect', 'x': 0, 'y': 0, 'w': 600, 'h': 110, 'fill': {'kind': 'linear', 'direction': 'horizontal', 'stops': [{'offset': 0, 'colour': '#172B45'}, {'offset': 1, 'colour': '#087F83'}]}},
        {'id': 'title', 'kind': 'text', 'x': 20, 'y': 20, 'w': 540, 'h': 80, 'size': 22, 'fill': '#FFFFFF', 'text': 'Creative technology\nLocal tools with human review'},
        {'id': 'landscape', 'kind': 'image', 'x': 20, 'y': 140, 'w': 320, 'h': 230, 'asset': 0, 'fit': 'cover'},
        {'id': 'logo', 'kind': 'image', 'x': 370, 'y': 150, 'w': 180, 'h': 180, 'asset': 1, 'fit': 'contain', 'opacity': .8},
        {'id': 'triangle', 'kind': 'polygon', 'x': 510, 'y': 330, 'w': 50, 'h': 40, 'points': [[0, 1], [.5, 0], [1, 1]], 'fill': '#D14936'}]}
    save('multi-image-gradient.json', scene)
    for ext in ['svg', 'png', 'pdf']:
        call('svg_compose', input='multi-image-gradient.json', output='multi-image-gradient.' + ext, assets=['original-landscape.jpg', 'transparent-logo.png'])

    if args.vision_model:
        for mode, image in [('describe', 'event-poster.png'), ('extract', 'event-poster.png'), ('design_review', 'event-poster.png'), ('document_review', 'review-business-proposal/page-001.png'), ('slide_review', 'review-business-presentation/page-002.png')]:
            value = call('vision_inspect', input=image, model=args.vision_model, mode=mode)
            if value:
                save('vision-' + mode + '.json', value)
        # Real rendered flawed scene -> real vision result -> consistency checks ->
        # separately approved bounded repair and new render. Never execute suggestions.
        flawed = deepcopy(scene)
        flawed['elements'][1]['fill'] = '#17405A'
        save('feedback-original.json', flawed)
        rendered = call('svg_compose', input='feedback-original.json', output='feedback-original.png', assets=['original-landscape.jpg', 'transparent-logo.png'])
        vision = call('vision_inspect', input='feedback-original.png', model=args.vision_model, mode='design_review')
        if rendered and vision:
            save('feedback-request.json', {'scene': flawed, 'link': {'render': rendered, 'vision': vision}})
            repair = call('scene_repair_plan', input='feedback-request.json')
            if repair:
                save('feedback-candidate.json', repair['scene'])
                call('svg_compose', input='feedback-candidate.json', output='feedback-candidate.png', assets=['original-landscape.jpg', 'transparent-logo.png'])
                report['feedback'] = repair
    report['passed'] = not report['failures']
    save('acceptance.json', report)
    print(json.dumps({'passed': report['passed'], 'operations': len(report['operations']), 'failures': len(report['failures']), 'output': str(root)}))
    return int(not report['passed'])


if __name__ == '__main__':
    raise SystemExit(main())
