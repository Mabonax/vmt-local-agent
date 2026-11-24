"""Embedded by the Rust broker. Not a shell or a general skill-script executor.

All paths below refer to a private invocation directory supplied by the broker.
No project path is accepted by this worker. The broker publishes fresh outputs.
This process runs with the user's privileges; validation is not an OS sandbox.
"""
import hashlib
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


def content(data, slides):
    if not isinstance(data, dict) or set(data) - {'title', 'subtitle', 'sections'}:
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
                text(value, 180 if slides else 4000)
                total += len(value)
            lines.extend(values)
        rows = section.get('table', [])
        if not isinstance(rows, list) or len(rows) > 100:
            raise ValueError('Invalid table')
        if slides and rows:
            raise ValueError('Baseline slides do not support tables')
        if rows:
            width = len(rows[0])
            if not 1 <= width <= 8:
                raise ValueError('Table must have 1-8 columns')
            for row in rows:
                if not isinstance(row, list) or len(row) != width:
                    raise ValueError('Unequal table rows')
                for value in row:
                    text(value, 1000)
                    total += len(value)
        if slides and (len(lines) > 5 or sum(map(len, lines)) > 450):
            raise ValueError('Dense slide: split content')
    if total > 30000:
        raise ValueError('Document text exceeds 30000 characters')


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


def format_existing(source, output, accent):
    """Patch only body/style XML. Preserve exact text and all opaque package parts."""
    from lxml import etree
    W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
    ns = {'w': W}
    tag = lambda name: '{' + W + '}' + name
    def child(parent, name, **attributes):
        element = parent.find(tag(name))
        if element is None:
            element = etree.SubElement(parent, tag(name))
        for key, value in attributes.items():
            element.set(tag(key), str(value))
        return element
    def font(properties, size, colour, bold=None):
        child(properties, 'rFonts', ascii='Arial', hAnsi='Arial', cs='Arial', eastAsia='Arial')
        child(properties, 'sz', val=size); child(properties, 'szCs', val=size)
        color = child(properties, 'color', val=colour)
        for key in ['themeColor', 'themeTint', 'themeShade']:
            color.attrib.pop(tag(key), None)
        if bold is not None:
            child(properties, 'b', val='1' if bold else '0')
    def all_text(data):
        root = etree.fromstring(data)
        return tuple((element.tag, element.text or '') for element in root.iter() if element.tag in [tag('t'), tag('instrText')])
    rgb = [int(accent[i:i+2], 16)/255 for i in (1,3,5)]
    luminance = sum(weight * (channel/12.92 if channel <= .04045 else ((channel+.055)/1.055)**2.4) for weight,channel in zip((.2126,.7152,.0722), rgb))
    ratio = 1.05/(luminance+.05)
    if ratio < 4.5:
        raise ValueError('Heading accent needs at least 4.5:1 contrast against white')
    with zipfile.ZipFile(source) as archive:
        document = etree.fromstring(archive.read('word/document.xml'))
        paragraphs = document.findall('.//w:body//w:p', ns)
        substantial_cv = len(paragraphs) >= 45 and sum(len(''.join(p.itertext(tag('t')))) for p in paragraphs) > 2000
        for paragraph in paragraphs:
            value = ''.join(paragraph.itertext(tag('t')))
            properties = paragraph.find(tag('pPr'))
            if properties is None:
                properties = etree.Element(tag('pPr')); paragraph.insert(0, properties)
            style = properties.find(tag('pStyle'))
            style_id = style.get(tag('val'), '') if style is not None else ''
            title = style_id == 'Title'
            heading = style_id.startswith('Heading')
            bullet = value.lstrip().startswith(('- ', '\u2022 ')) or properties.find(tag('numPr')) is not None
            role = ' | ' in value and len(value) < 135 and not value.startswith('Email:')
            size = 48 if title else 24 if heading else 21
            colour = '000000' if title else accent[1:] if heading else '263445'
            child(properties, 'spacing', before=180 if heading else 0, after=80 if value.strip() else 0, line=250 if value.strip() else 80, lineRule='auto')
            child(properties, 'widowControl', val='1')
            child(properties, 'keepNext', val='1' if title or heading or role else '0')
            # Give substantial CVs a deliberate section boundary instead of a
            # short second page beginning midway through Projects.
            if substantial_cv and heading and value.strip().casefold() == 'education':
                child(properties, 'pageBreakBefore', val='1')
            child(properties, 'ind', left=180 if bullet else 0, hanging=180 if bullet else 0, right=0)
            for border in list(properties.findall(tag('pBdr'))):
                properties.remove(border)
            for run in paragraph.findall('.//w:r', ns):
                rp = run.find(tag('rPr'))
                if rp is None:
                    rp = etree.Element(tag('rPr')); run.insert(0, rp)
                font(rp, size, colour, True if title or heading or role else None)
        for section in document.findall('.//w:sectPr', ns):
            child(section, 'pgSz', w=11906, h=16838)
            child(section, 'pgMar', top=1134, bottom=1134, left=1134, right=1134)
        styles = etree.fromstring(archive.read('word/styles.xml'))
        for style in styles.findall('w:style', ns):
            name = style.get(tag('styleId'), '')
            if name in ['Normal','Title','Subtitle','Heading1','Heading2','ListBullet']:
                font(child(style, 'rPr'), 48 if name == 'Title' else 24 if name.startswith('Heading') else 21, '000000' if name == 'Title' else accent[1:] if name.startswith('Heading') else '263445')
                for border in style.findall('w:pPr/w:pBdr', ns):
                    border.getparent().remove(border)
        changes = {'word/document.xml': etree.tostring(document, xml_declaration=True, encoding='UTF-8', standalone=True), 'word/styles.xml': etree.tostring(styles, xml_declaration=True, encoding='UTF-8', standalone=True)}
        if all(archive.read(name) == data for name,data in changes.items()):
            raise ValueError('No formatting change was produced')
        if all_text(archive.read('word/document.xml')) != all_text(changes['word/document.xml']):
            raise ValueError('Formatting changed source content')
        with zipfile.ZipFile(output, 'w') as destination:
            for item in archive.infolist():
                destination.writestr(item, changes.get(item.filename, archive.read(item)))
    with zipfile.ZipFile(source) as before, zipfile.ZipFile(output) as after:
        if before.namelist() != after.namelist() or any(before.read(name) != after.read(name) for name in before.namelist() if name not in changes):
            raise ValueError('Formatting changed an opaque package part')
    return {'content_preserved': True, 'preserved_other_package_parts': True, 'accent': accent.upper(), 'accent_contrast_on_white': round(ratio,4), 'preset': 'cv'}


def run(request):
    args = request['args']
    operation = args['operation']
    namespace = {'__name__': '_reviewed_design_skill'}
    if operation in ['inspect_document', 'format_document']:
        source = Path('input')
        validate_office(source)
        if operation == 'inspect_document':
            from docx import Document
            document = Document(source)
            paragraphs = [{'style':p.style.name,'text':p.text} for p in document.paragraphs]
            tables = [[[cell.text for cell in row.cells] for row in table.rows] for table in document.tables]
            result = {'operation':operation,'input':args['input'],'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'paragraphs':paragraphs,'tables':tables,'paragraph_count':len(paragraphs),'table_count':len(tables),'limitation':'Body paragraphs and tables only; images, textboxes, headers and footers remain preserved but are not described.'}
            if len(json.dumps(result).encode()) > 30000:
                raise ValueError('Document inspection exceeds text limit; narrow the document')
            return result
        output = Path('artifact.docx')
        result = format_existing(source, output, args.get('accent','#087F83'))
        result.update({'operation':operation,'input':args['input'],'created':args['output'],'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'visual_review':'PENDING'})
        if result['source_sha256'] == result['sha256']:
            raise ValueError('Formatting output is identical to source')
        return result
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
        result = {'artifact_sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'pages': len(pdf), 'render_status': 'PASS', 'visual_review': 'PENDING', 'review_directory': args['output']}
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
