"""Prepend a cover without reconstructing the CV or changing other ZIP parts."""
import hashlib
import io
import zipfile
from copy import copy,deepcopy
from lxml import etree as ET

W='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
NS={'w':W}

def add_cover(source):
    from docx import Document
    from docx.enum.text import WD_BREAK
    from docx.shared import Pt,RGBColor
    from docx.oxml import OxmlElement
    from docx.oxml.ns import qn
    with zipfile.ZipFile(io.BytesIO(source)) as archive:
        if len(set(archive.namelist()))!=len(archive.infolist()):raise ValueError('Duplicate Office package parts')
        if any(name.lower().startswith('_xmlsignatures/') for name in archive.namelist()):raise ValueError('Signed documents require a separately reviewed unsigned copy')
        tree=ET.fromstring(archive.read('word/document.xml'),ET.XMLParser(resolve_entities=False,no_network=True))
        body=tree.find('w:body',NS)
        if body is None:raise ValueError('Missing Word body')
        original=[ET.tostring(node) for node in body]
        texts=[''.join(p.xpath('.//w:t/text()',namespaces=NS)).strip() for p in body.findall('w:p',NS)]
        texts=[text for text in texts if text]
        if not texts:raise ValueError('Cover requires readable existing body text')
        name=next((text for text in texts[:12] if 'NAME' in text.upper() and '[' in text and len(text)<=120),None)
        if name is None:
            for i,text in enumerate(texts[:8]):
                if text.casefold() in ('contact information','contact details') and i+1<len(texts):
                    name=texts[i+1];break
        if name is None:
            name=next((t for t in texts[:4] if len(t)<=120 and not any(x in t.casefold() for x in ('curriculum','template','resume','résumé','contact','cv'))),'')
        contacts=[]
        for text in texts[:12]:
            if any(tag in text.casefold() for tag in ('email:','phone:','linkedin:','location:')):
                contacts.extend(part.strip() for part in text.split('|') if part.strip())
        cover=Document()
        def paragraph(text,size,bold=False,before=0,after=12,title=False):
            p=cover.add_paragraph(style='Title' if title else 'Normal')
            fmt=p.paragraph_format
            fmt.space_before=Pt(before);fmt.space_after=Pt(after);fmt.line_spacing=1.15
            fmt.left_indent=Pt(0);fmt.right_indent=Pt(0);fmt.first_line_indent=Pt(0)
            fmt.page_break_before=False;fmt.keep_with_next=True
            borders=OxmlElement('w:pBdr')
            for side in ('top','left','bottom','right','between','bar'):
                edge=OxmlElement('w:'+side);edge.set(qn('w:val'),'nil');borders.append(edge)
            p._p.get_or_add_pPr().append(borders)
            r=p.add_run(text);r.font.name='Calibri';r.font.size=Pt(size);r.bold=bold;r.italic=False;r.underline=False;r.font.color.rgb=RGBColor(0,0,0)
            return p
        paragraph('Curriculum Vitae',26,True,before=110,after=22,title=True)
        if name:paragraph(name[:120],30,True,after=16)
        paragraph('Professional profile and experience',12,after=38)
        for line in contacts[:4]:paragraph(line[:240],11,after=8)
        end=paragraph('',1,after=0);end.paragraph_format.keep_with_next=False
        end.add_run().add_break(WD_BREAK.PAGE)
        prefix=[deepcopy(p._p) for p in cover.paragraphs]
        for index,node in enumerate(prefix):body.insert(index,node)
        if [ET.tostring(node) for node in list(body)[len(prefix):]]!=original:raise ValueError('Original body changed')
        replacement=ET.tostring(tree,xml_declaration=True,encoding='UTF-8',standalone=True)
        output=io.BytesIO()
        with zipfile.ZipFile(output,'w') as result:
            for item in archive.infolist():result.writestr(copy(item),replacement if item.filename=='word/document.xml' else archive.read(item))
        raw=output.getvalue()
        if len(raw)>700000:raise ValueError('Covered document exceeds artifact limit')
        # Independently reopen the result and verify untouched package contents.
        with zipfile.ZipFile(io.BytesIO(raw)) as result:
            if any(result.read(item)!=archive.read(item) for item in archive.namelist() if item!='word/document.xml'):raise ValueError('Original package parts changed')
    Document(io.BytesIO(raw))
    return raw,{'source_sha256':hashlib.sha256(source).hexdigest(),'original_body_elements':len(original),'original_body_preserved':True,'unchanged_package_parts':True,'cover_identity':'copied from existing document; no invented personal details'}
