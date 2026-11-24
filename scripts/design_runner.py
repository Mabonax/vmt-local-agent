"""Embedded by the Rust broker. Not a shell or a general skill-script executor.

All paths below refer to a private invocation directory supplied by the broker.
No project path is accepted by this worker. The broker publishes fresh outputs.
This process runs with the user's privileges; validation is not an OS sandbox.
"""
import hashlib
import os
import json
from pathlib import Path
import re
import subprocess
import sys
import warnings
import zipfile
import xml.etree.ElementTree as ET


def text(value, limit=4000):
    if not isinstance(value, str) or len(value) > limit or any(ord(c) < 32 and c not in '\n\t' for c in value) or '\ufffd' in value:
        raise ValueError('Invalid or oversized text')
    return value


def content(data, slides, v2=False):
    if not isinstance(data, dict) or set(data) - ({'title', 'subtitle', 'sections', 'design_system'} if v2 else {'title', 'subtitle', 'sections'}):
        raise ValueError('Unsupported document fields')
    text(data['title'], 75 if slides else 200)
    if not data['title'].strip():
        raise ValueError('Title is required')
    text(data.get('subtitle', ''), 180 if slides else 500)
    sections = data['sections']
    if not isinstance(sections, list) or not 1 <= len(sections) <= 30:
        raise ValueError('Provide 1-30 sections')
    total = 0
    for section in sections:
        if not isinstance(section, dict) or set(section) - {'heading', 'paragraphs', 'bullets', 'table'}:
            raise ValueError('Unsupported section fields')
        text(section['heading'], 75 if slides else 200)
        lines = []
        for key in ['paragraphs', 'bullets']:
            values = section.get(key, [])
            if not isinstance(values, list) or len(values) > 100:
                raise ValueError('Invalid paragraph/bullet list')
            for value in values:
                text(value, 180 if slides and not v2 else 4000)
                total += len(value)
            lines.extend(values)
        rows = section.get('table', [])
        if not isinstance(rows, list) or len(rows) > 100:
            raise ValueError('Invalid table')
        if slides and rows and not v2:
            raise ValueError('Baseline slides do not support tables')
        if rows:
            if slides and v2 and len(rows) < 2:
                raise ValueError('Slide table requires a header and at least one data row')
            width = len(rows[0])
            if not 1 <= width <= 8:
                raise ValueError('Table must have 1-8 columns')
            for row in rows:
                if not isinstance(row, list) or len(row) != width:
                    raise ValueError('Unequal table rows')
                for value in row:
                    text(value, 1000)
                    total += len(value)
        if slides and not v2 and (len(lines) > 5 or sum(map(len, lines)) > 450):
            raise ValueError('Dense slide: split content')
    if total > 30000:
        raise ValueError('Document text exceeds 30000 characters')


def design_system(data, slides):
    """Only approved scalar design tokens; no executable paths or unknown fields."""
    raw = data.get('design_system', {})
    allowed = {'version', 'colour', 'typography', 'layout', 'quality'}
    if not isinstance(raw, dict) or set(raw) - allowed:
        raise ValueError('Unknown design system fields')
    if raw.get('version', '2.0') != '2.0':
        raise ValueError('Unsupported design system version')
    rules = {
        'colour': {'primary', 'accent', 'background', 'text', 'surface', 'muted_text', 'on_primary', 'on_accent', 'brand_seed'},
        'typography': {'font', 'body_size', 'slide_body_size'},
        'layout': {'page_margin_mm'},
        'quality': {'maximum_repair_attempts'},
    }
    for group, keys in rules.items():
        section = raw.get(group, {})
        if not isinstance(section, dict) or set(section) - keys:
            raise ValueError('Unknown design token')
    for colour in raw.get('colour', {}).values():
        if not isinstance(colour, str) or not re.fullmatch(r'#[0-9a-fA-F]{6}', colour):
            raise ValueError('Invalid colour token')
    def number(group, key, default, lo, hi):
        value = raw.get(group, {}).get(key, default)
        if type(value) not in (int, float) or not lo <= value <= hi:
            raise ValueError('Design number outside supported range')
        return value
    number('typography', 'body_size', 11, 10, 16)
    number('typography', 'slide_body_size', 23, 18, 28)
    number('layout', 'page_margin_mm', 22, 15, 30)
    attempts = number('quality', 'maximum_repair_attempts', 3, 1, 3)
    if type(attempts) is not int:
        raise ValueError('Repair attempts must be an integer')
    typography = raw.setdefault('typography', {})
    typography.setdefault('font', 'Arial' if os.name == 'nt' else 'Liberation Sans')
    if typography['font'] not in ('Arial', 'Liberation Sans'):
        raise ValueError('Font is not in the reviewed font allowlist')
    if not slides and raw.get('colour', {}).get('background', '#FFFFFF').upper() != '#FFFFFF':
        raise ValueError('V2 Word currently supports white page backgrounds only')
    data['design_system'] = raw
    return raw


def reviewed_font(name):
    if name not in ('Arial', 'Liberation Sans'):
        raise ValueError('Font is not in the reviewed font allowlist')
    if os.name == 'nt':
        root = Path(os.environ['SystemRoot']) / 'Fonts'
        # Match the actual font used by the native Office object.
        if name != 'Arial':
            raise ValueError('Use Arial for the Windows design runtime')
        path = root / 'arial.ttf'
        if not path.is_file():
            raise ValueError('Required Arial font is unavailable')
        return str(path)
    return subprocess.check_output(['fc-match', '-f', '%{file}', name], text=True, timeout=10).strip()


def structural_review(pdf):
    # Geometric evidence, not aesthetic or accessibility certification.
    findings = []
    pages = []
    for index, page in enumerate(pdf):
        spans = [s for b in page.get_text('dict')['blocks'] if 'lines' in b
                 for line in b['lines'] for s in line['spans'] if s['text'].strip()]
        pages.append({'page': index + 1, 'text_spans': len(spans)})
        if len(spans) > 1000:
            findings.append({'page': index + 1, 'type': 'inspection_limit'})
            continue
        if not spans:
            findings.append({'page': index + 1, 'type': 'no_extractable_text'})
        import fitz
        for i, span in enumerate(spans):
            a = fitz.Rect(span['bbox'])
            if a.x0 < -0.5 or a.y0 < -0.5 or a.x1 > page.rect.width + 0.5 or a.y1 > page.rect.height + 0.5:
                findings.append({'page': index + 1, 'type': 'text_out_of_bounds'})
            for other in spans[i + 1:]:
                b = fitz.Rect(other['bbox'])
                area = min(a.get_area(), b.get_area())
                if area > 0 and (a & b).get_area() > 0.35 * area:
                    findings.append({'page': index + 1, 'type': 'possible_text_overlap'})
                if len(findings) >= 100:
                    return {'pages': pages, 'issues': findings, 'structural_status': 'REVIEW_REQUIRED', 'truncated': True}
    return {'pages': pages, 'issues': findings, 'structural_status': 'PASS' if not findings else 'REVIEW_REQUIRED'}


def validate_office(path):
    with zipfile.ZipFile(path) as archive:
        entries = archive.infolist()
        if len(entries) > 2000 or sum(i.file_size for i in entries) > 64 * 1024 * 1024:
            raise ValueError('Office archive exceeds limits')
        for item in entries:
            name = item.filename.lower()
            if any(part == '..' for part in name.split('/')) or name.startswith('/') or '\\' in name or ':' in name:
                raise ValueError('Invalid office archive path')
            if any(marker in name for marker in ['vbaproject', '/embeddings/', '/activex/']):
                raise ValueError('Embedded code and objects are unsupported')
            if name.endswith(('.xml', '.rels')):
                data = archive.read(item)
                if b'<!DOCTYPE' in data.upper() or b'<!ENTITY' in data.upper():
                    raise ValueError('XML entities are unsupported')
                root = ET.fromstring(data)
                if name.endswith('.rels') and any(e.get('TargetMode', '').lower() == 'external' for e in root):
                    raise ValueError('External office relationships are unsupported')
                # Do not allow fields to fetch external content during conversion.
                for element in root.iter():
                    if element.tag.rsplit('}', 1)[-1] in ['instrText', 'fldSimple']:
                        instruction = (element.text or '') + element.get('{http://schemas.openxmlformats.org/wordprocessingml/2006/main}instr', '')
                        if re.search(r'INCLUDE|DDE|LINK|DATABASE', instruction, re.I):
                            raise ValueError('External document fields are unsupported')


def contrast_ratio(a, b):
    def luminance(h):
        channels = [int(h[i:i+2], 16) / 255 for i in (1, 3, 5)]
        linear = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4 for v in channels]
        return sum(v * w for v, w in zip(linear, (.2126, .7152, .0722)))
    a, b = sorted((luminance(a), luminance(b)))
    return (b + .05) / (a + .05)


def repair_text_roles(theme):
    changes = []
    colours = theme['colour']
    for role in ('text', 'primary'):
        original = colours[role]
        if contrast_ratio(original, colours['background']) < 4.5:
            colours[role] = max(('#000000', '#FFFFFF'), key=lambda c: contrast_ratio(c, colours['background']))
            changes.append({'role': role, 'before': original, 'after': colours[role]})
    return changes


def render_pdf(source, soffice, timeout):
    profile = Path('office-profile').resolve()
    pdf = source.with_suffix('.pdf')
    if pdf.exists():
        pdf.unlink()
    subprocess.run([soffice, f'-env:UserInstallation={profile.as_uri()}', '--headless',
                    '--convert-to', 'pdf', '--outdir', '.', str(source.resolve())],
                   check=True, timeout=timeout, capture_output=True)
    if not pdf.is_file():
        raise ValueError('Renderer did not create PDF')
    return pdf


def compose_checked(request, data, namespace, theme, output, slides):
    import time
    import fitz
    deadline = time.monotonic() + 105
    repairs = repair_text_roles(theme)
    attempts = []
    maximum = theme['quality']['maximum_repair_attempts'] if slides else 1
    for attempt in range(maximum):
        if slides:
            namespace['slides'](data, output, theme, 1 - attempt * .2)
            from pptx import Presentation
            from pptx.dml.color import RGBColor
            deck = Presentation(output)
            if len(deck.slides) > 40:
                raise ValueError('V2 slide count exceeds 40')
            for slide in deck.slides:
                for shape in slide.shapes:
                    if shape.has_table:
                        for row in shape.table.rows:
                            for cell in row.cells:
                                cell.fill.solid()
                                cell.fill.fore_color.rgb = RGBColor.from_string(theme['colour']['background'][1:])
            deck.save(output)
        else:
            namespace['word'](data, output, theme)
            # The upstream header fill is fixed; ensure it shares the checked background.
            from docx import Document
            from docx.oxml.ns import qn
            doc = Document(output)
            for table in doc.tables:
                for cell in table.rows[0].cells:
                    for shading in cell._tc.iter(qn('w:shd')):
                        shading.set(qn('w:fill'), theme['colour']['background'][1:])
            doc.save(output)
        validate_office(output)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ValueError('Composition exceeded time budget')
        with fitz.open(render_pdf(output, request['soffice'], remaining)) as pdf:
            if not 1 <= len(pdf) <= 40:
                raise ValueError('Render must have 1-40 pages/slides')
            report = structural_review(pdf)
            attempts.append({'attempt': attempt + 1, 'pages': len(pdf),
                             'issues': report['issues'], 'status': report['structural_status']})
        if report['structural_status'] == 'PASS':
            return {'created': request['args']['output'], 'sha256': hashlib.sha256(output.read_bytes()).hexdigest(),
                    'visual_review': 'PENDING', 'structural_status': 'PASS', 'attempts': attempts,
                    'colour_repairs': repairs, 'limitation': 'Text geometry checks only; run render for visual review images.'}
    # Broker publishes only successful results. Never release a known failed candidate.
    raise ValueError('Structural review failed after bounded composition attempts')


def type_scale(mode, size):
    if mode not in ('document', 'presentation') or type(size) is not int or not 10 <= size <= 28:
        raise ValueError('Invalid type scale')
    if mode == 'document':
        return {'title_pt': round(size * 2.5, 1), 'heading1_pt': round(size * 1.65, 1),
                'heading2_pt': round(size * 1.3, 1), 'body_pt': size,
                'caption_pt': max(9, round(size * .82, 1)), 'line_height': 1.25,
                'paragraph_after_pt': round(size * .6, 1), 'guidance': 'Prefer roughly 45–80 characters per line; adapt to language and medium.'}
    return {'title_pt': round(size * 1.8, 1), 'heading_pt': round(size * 1.35, 1),
            'body_pt': size, 'caption_pt': max(12, round(size * .7, 1)),
            'line_height': 1.2, 'guidance': 'Split crowded slides rather than shrink body text.'}


def measure_text(font, sample, size, width):
    from PIL import ImageFont
    text(sample, 4096)
    if not sample.strip() or type(size) is not int or not 8 <= size <= 72 or type(width) is not int or not 40 <= width <= 1200:
        raise ValueError('Invalid measurement input')
    # Measure at 288 DPI (four pixels per point) to reduce rounding.
    face = ImageFont.truetype(reviewed_font(font), size * 4)
    measure = lambda value: face.getlength(value) / 4
    lines = []
    oversized = False
    for paragraph in sample.splitlines():
        current = ''
        for word in paragraph.split():
            candidate = (current + ' ' + word).strip()
            if measure(candidate) <= width:
                current = candidate
            else:
                if current:
                    lines.append(current)
                current = word
                oversized |= measure(word) > width
        lines.append(current)
    ascent, descent = face.getmetrics()
    return {'font': font, 'size_pt': size, 'available_width_pt': width,
            'line_count': len(lines), 'max_line_width_pt': round(max(map(measure, lines)), 2),
            'estimated_height_pt': round(len(lines) * (ascent + descent) / 4, 2),
            'unbreakable_word_overflow': oversized,
            'limitation': 'Plain-text whitespace wrapping estimate, not native Office layout or glyph-coverage certification.'}


def bounded_number(value, low, high):
    if type(value) not in (int, float) or not low <= value <= high:
        raise ValueError('Number outside supported bounds')
    return value


def layout_grid(data):
    if not isinstance(data, dict) or set(data) - {'width', 'height', 'margin', 'gutter', 'columns', 'rows'}:
        raise ValueError('Invalid grid fields')
    w = bounded_number(data['width'], 100, 4000)
    h = bounded_number(data['height'], 100, 4000)
    m = bounded_number(data.get('margin', 24), 0, 500)
    g = bounded_number(data.get('gutter', 12), 0, 100)
    c, r = data.get('columns', 12), data.get('rows', 1)
    if type(c) is not int or type(r) is not int or not 1 <= c <= 12 or not 1 <= r <= 12:
        raise ValueError('Grid dimensions must be integers 1-12')
    cw, ch = (w - 2*m - (c-1)*g)/c, (h - 2*m - (r-1)*g)/r
    if min(cw, ch) < 4:
        raise ValueError('Grid has insufficient usable space')
    return {'unit': 'pt', 'width': w, 'height': h, 'margin': m, 'gutter': g,
            'cells': [{'row': y+1, 'column': x+1, 'x': round(m+x*(cw+g), 4),
                       'y': round(m+y*(ch+g), 4), 'w': round(cw, 4), 'h': round(ch, 4)}
                      for y in range(r) for x in range(c)]}


def layout_check(data):
    if not isinstance(data, dict) or set(data) - {'width', 'height', 'margin', 'elements'}:
        raise ValueError('Invalid layout fields')
    w = bounded_number(data['width'], 100, 4000)
    h = bounded_number(data['height'], 100, 4000)
    m = bounded_number(data.get('margin', 0), 0, 500)
    if 2*m >= min(w,h):
        raise ValueError('Invalid safe margin')
    elements = data['elements']
    if not isinstance(elements, list) or not 1 <= len(elements) <= 64:
        raise ValueError('Provide 1-64 elements')
    seen, issues = set(), []
    for e in elements:
        if not isinstance(e, dict) or set(e) != {'id', 'x', 'y', 'w', 'h'}:
            raise ValueError('Invalid element fields')
        name = e['id']
        if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z0-9_-]{1,48}', name) or name in seen:
            raise ValueError('Invalid or duplicate element ID')
        seen.add(name)
        for k in ('x','y'):
            bounded_number(e[k], -4000, 4000)
        for k in ('w','h'):
            bounded_number(e[k], .1, 4000)
        if e['x'] < m or e['y'] < m or e['x']+e['w'] > w-m+.0001 or e['y']+e['h'] > h-m+.0001:
            issues.append({'type': 'outside_safe_area', 'id': name})
    total = len(issues)
    for i,a in enumerate(elements):
        for b in elements[i+1:]:
            dx = min(a['x']+a['w'],b['x']+b['w'])-max(a['x'],b['x'])
            dy = min(a['y']+a['h'],b['y']+b['h'])-max(a['y'],b['y'])
            if dx > .001 and dy > .001:
                total += 1
                if len(issues) < 100:
                    issues.append({'type': 'overlap', 'id': a['id'], 'other': b['id'], 'area_pt2': round(dx*dy, 3)})
    return {'status': 'PASS' if total == 0 else 'REVIEW_REQUIRED', 'issue_count': total,
            'issues': issues[:100], 'truncated': total > 100,
            'limitation': 'Axis-aligned rectangles only; intentional layering and text fit require separate review.'}


def run(request):
    args = request['args']
    operation = args['operation']
    if operation in ('layout_grid', 'layout_check'):
        data = json.loads(Path('input').read_text(encoding='utf-8-sig'))
        return layout_grid(data) if operation == 'layout_grid' else layout_check(data)
    if operation == 'type_scale':
        return type_scale(args['mode'], args.get('size', 12))
    if operation == 'measure_text':
        return measure_text(args['font'], args['sample'], args.get('size', 12), args.get('width', 400))
    namespace = {'__name__': '_reviewed_design_skill'}
    if operation in ['repair', 'gradient', 'categorical', 'simulate', 'system', 'distance']:
        # Both sources are pinned to the same reviewed package by the broker.
        # Execute the base in a private namespace; no sys.path or filesystem imports.
        base = dict(namespace)
        exec(compile(request['colour_base'], '<reviewed-colour-base>', 'exec'), base)
        advanced = request['script']
        if advanced.count('from colour import *') != 1:
            raise ValueError('Unexpected advanced import contract')
        namespace.update({k: v for k, v in base.items() if not k.startswith('_')})
        exec(compile(advanced.replace('from colour import *', ''), '<reviewed-colour-v2>', 'exec'), namespace)
        if operation == 'repair':
            return namespace['repair'](args['foreground'], args['background'], 4.5)
        if operation == 'gradient':
            return namespace['gradient'](args['foreground'], args['background'], args.get('count', 7))
        if operation == 'categorical':
            return namespace['categorical'](args['seed'], args.get('count', 6))
        if operation == 'simulate':
            return namespace['simulate'](args['seed'], args['mode'])
        if operation == 'system':
            return namespace['system'](args['seed'], args.get('theme', 'light'))
        return {'distance_oklab': namespace['distance'](args['foreground'], args['background'])}
    if operation in ['palette', 'contrast', 'extract']:
        exec(compile(request['script'], '<reviewed-colour-script>', 'exec'), namespace)
        if operation == 'palette':
            return namespace['palette'](args['seed'], args.get('mode', 'analogous'), args.get('theme', 'light'))
        if operation == 'contrast':
            return namespace['audit'](args['foreground'], args['background'])
        from PIL import Image
        Image.MAX_IMAGE_PIXELS = 16_000_000
        warnings.simplefilter('error', Image.DecompressionBombWarning)
        with Image.open('input', formats=['PNG', 'JPEG', 'WEBP']) as image:
            if image.width * image.height > 16_000_000:
                raise ValueError('Image exceeds pixel limit')
            image.load()
            image.save('validated.png')
        return namespace['extract']('validated.png', args.get('count', 6))
    if operation == 'compose_v2':
        data = json.loads(Path('input').read_text(encoding='utf-8-sig'))
        slides = args['output'].lower().endswith('.pptx')
        content(data, slides, v2=True)
        design_system(data, slides)
        exec(compile(request['script'], '<reviewed-document-v2>', 'exec'), namespace)
        namespace['fontfile'] = reviewed_font
        t = namespace['theme'](data)
        output = Path('artifact.pptx' if slides else 'artifact.docx')
        return compose_checked(request, data, namespace, t, output, slides)
    if operation == 'compose':
        data = json.loads(Path('input').read_text(encoding='utf-8-sig'))
        slides = args['output'].lower().endswith('.pptx')
        content(data, slides)
        exec(compile(request['script'], '<reviewed-document-composer>', 'exec'), namespace)
        output = Path('artifact.pptx' if slides else 'artifact.docx')
        namespace['slides' if slides else 'word'](data, output)
        if not slides:
            # Remove the library template's title rule and align its inherited
            # subtitle/table accent with the supplied navy/teal design standard.
            from docx import Document
            from docx.shared import Pt, RGBColor
            from docx.oxml.ns import qn
            from lxml import etree
            document = Document(output)
            for style_name in ['Title', 'Subtitle']:
                style = document.styles[style_name]
                properties = style.element.pPr
                if properties is not None:
                    for border in list(properties.findall(qn('w:pBdr'))):
                        properties.remove(border)
            subtitle = document.styles['Subtitle']
            subtitle.font.name = 'Liberation Sans'
            subtitle.font.size = Pt(12)
            subtitle.font.italic = False
            subtitle.font.color.rgb = RGBColor.from_string('263445')
            for element in document.styles.element.iter():
                if element.get(qn('w:themeColor')) == 'accent1' or element.get(qn('w:color')) == '4F81BD':
                    element.set(qn('w:color'), '087F83')
                if element.get(qn('w:themeFill')) == 'accent1':
                    element.set(qn('w:fill'), 'D8EDEC' if element.get(qn('w:themeFillTint')) else '087F83')
            for part in document.part.package.parts:
                if str(part.partname) == '/word/theme/theme1.xml':
                    theme = etree.fromstring(part.blob)
                    accent = theme.find('.//{http://schemas.openxmlformats.org/drawingml/2006/main}accent1/{http://schemas.openxmlformats.org/drawingml/2006/main}srgbClr')
                    if accent is not None:
                        accent.set('val', '087F83')
                    part._blob = etree.tostring(theme)
            document.save(output)
        return {'created': args['output'], 'sha256': hashlib.sha256(output.read_bytes()).hexdigest(), 'visual_review': 'PENDING', 'limitation': 'Baseline editable text, bullets and Word tables; no custom charts or layouts.'}
    if operation != 'render':
        raise ValueError('Unregistered operation')
    import fitz
    source = Path('input' + Path(args['input']).suffix.lower())
    source.write_bytes(Path('input').read_bytes())
    validate_office(source)
    profile = Path('office-profile').resolve()
    subprocess.run([request['soffice'], f'-env:UserInstallation={profile.as_uri()}', '--headless', '--convert-to', 'pdf', '--outdir', '.', str(source.resolve())], check=True, timeout=85, capture_output=True)
    review = Path('review')
    review.mkdir()
    with fitz.open(source.with_suffix('.pdf')) as pdf:
        if not 1 <= len(pdf) <= 40:
            raise ValueError('Render must have 1-40 pages/slides')
        for index, page in enumerate(pdf):
            if page.rect.width * page.rect.height > 2_000_000:
                raise ValueError('Page dimensions exceed limits')
            page.get_pixmap(matrix=fitz.Matrix(1.7, 1.7)).save(review / f'page-{index+1:03}.png')
        result = {'artifact_sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'pages': len(pdf), 'render_status': 'PASS', 'visual_review': 'PENDING', 'review_directory': args['output'], **{'structural_review': structural_review(pdf)}}
    (review / 'review.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    return result


if __name__ == '__main__':
    try:
        payload = json.loads(sys.stdin.buffer.read(100_001))
        print(json.dumps(run(payload), ensure_ascii=True))
    except Exception as error:
        # Do not echo source data or arbitrary exceptions containing content.
        print(json.dumps({'error': type(error).__name__, 'message': 'Design operation failed validation or execution; inspect inputs and local dependencies.'}))
        sys.exit(1)
