"""Prepend a cover without reconstructing the CV or changing other ZIP parts."""
import hashlib
import io
import zipfile
from copy import copy,deepcopy
from lxml import etree as ET

W='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
NS={'w':W}

def add_cover(source,design=None):
    from docx import Document
    from docx.enum.text import WD_BREAK
    from docx.shared import Pt,RGBColor
    from docx.oxml import OxmlElement
    from docx.oxml.ns import qn
    design=design or {'primary':'#172B45','accent':'#087F83','text':'#263445','surface':'#EDF5F5','background':'#FFFFFF','font':'Liberation Sans'}
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
        def paragraph(text,size,bold=False,before=0,after=12,title=False,colour=None):
            p=cover.add_paragraph(style='Title' if title else 'Normal')
            fmt=p.paragraph_format
            fmt.space_before=Pt(before);fmt.space_after=Pt(after);fmt.line_spacing=1.15
            fmt.left_indent=Pt(0);fmt.right_indent=Pt(0);fmt.first_line_indent=Pt(0)
            fmt.page_break_before=False;fmt.keep_with_next=True
            borders=OxmlElement('w:pBdr')
            for side in ('top','left','bottom','right','between','bar'):
                edge=OxmlElement('w:'+side);edge.set(qn('w:val'),'nil');borders.append(edge)
            p._p.get_or_add_pPr().append(borders)
            r=p.add_run(text);r.font.name=design['font'];r.font.size=Pt(size);r.bold=bold;r.italic=False;r.underline=False;r.font.color.rgb=RGBColor.from_string((colour or design['text']).lstrip('#'))
            return p
        title=paragraph('Curriculum Vitae',30,True,before=100,after=24,title=True,colour='#000000')
        if name:
            identity=paragraph(name[:120],36,True,after=24,colour='#FFFFFF')
            properties=identity._p.get_or_add_pPr()
            shade=OxmlElement('w:shd');shade.set(qn('w:fill'),design['primary'].lstrip('#'));properties.append(shade)
            for old in list(properties.findall(qn('w:pBdr'))):properties.remove(old)
            frame=OxmlElement('w:pBdr')
            for side in ('top','bottom','left','right'):
                edge=OxmlElement('w:'+side);edge.set(qn('w:val'),'single');edge.set(qn('w:sz'),'72');edge.set(qn('w:space'),'12');edge.set(qn('w:color'),design['accent' if side=='left' else 'primary'].lstrip('#'));frame.append(edge)
            properties.append(frame)
        paragraph('Professional profile and experience',12,after=40)
        # An editable document badge complements the native navy name panel.
        # It is anchored only to the inserted first page.
        section=body.find('w:sectPr',NS);page=section.find('w:pgSz',NS) if section is not None else None
        page_width=int(page.get('{'+W+'}w','12240'))/20 if page is not None else 612
        page_height=int(page.get('{'+W+'}h','15840'))/20 if page is not None else 792
        v='urn:schemas-microsoft-com:vml'
        pict=ET.Element('{'+W+'}pict')
        existing_ids=set(tree.xpath('//@id'))
        def rectangle(x,y,width,height,fill,label=None):
            rect=ET.SubElement(pict,'{'+v+'}rect',nsmap={'v':v})
            index=len(existing_ids)+1
            while f'vmt_cover_{index}' in existing_ids:index+=1
            identity=f'vmt_cover_{index}';existing_ids.add(identity);rect.set('id',identity)
            rect.set('style',f'position:absolute;margin-left:{x}pt;margin-top:{y}pt;width:{width}pt;height:{height}pt;z-index:1;mso-position-horizontal-relative:page;mso-position-vertical-relative:page')
            rect.set('fillcolor',fill);rect.set('stroked','f')
            if label:
                box=ET.SubElement(rect,'{'+v+'}textbox');box.set('inset','0,0,0,0')
                content=ET.SubElement(box,'{'+W+'}txbxContent');p=ET.SubElement(content,'{'+W+'}p');pr=ET.SubElement(p,'{'+W+'}pPr')
                ET.SubElement(pr,'{'+W+'}jc').set('{'+W+'}val','center');sp=ET.SubElement(pr,'{'+W+'}spacing');sp.set('{'+W+'}before','190');sp.set('{'+W+'}after','0')
                r=ET.SubElement(p,'{'+W+'}r');rp=ET.SubElement(r,'{'+W+'}rPr');ET.SubElement(rp,'{'+W+'}color').set('{'+W+'}val','FFFFFF');ET.SubElement(rp,'{'+W+'}sz').set('{'+W+'}val','48');ET.SubElement(rp,'{'+W+'}rFonts').set('{'+W+'}ascii',design['font']);ET.SubElement(r,'{'+W+'}t').text=label
        rectangle(page_width-135,68,52,52,design['primary'],'CV')
        title.add_run()._r.append(pict)
        paragraph('Contact details',11,True,after=12,colour=design['accent'])
        if contacts:
            table=cover.add_table(rows=0,cols=2);table.autofit=False
            from docx.shared import Inches
            table.columns[0].width=Inches(1.05);table.columns[1].width=Inches(4.6)
            borders=OxmlElement('w:tblBorders')
            for side in ('top','left','bottom','right','insideH','insideV'):
                edge=OxmlElement('w:'+side);edge.set(qn('w:val'),'nil');borders.append(edge)
            table._tbl.tblPr.append(borders)
            for line in contacts[:4]:
                label,sep,value=line.partition(':');row=table.add_row();row._tr.get_or_add_trPr().append(OxmlElement('w:cantSplit'))
                for index,cell in enumerate(row.cells):
                    shade=OxmlElement('w:shd');shade.set(qn('w:fill'),design['surface'].lstrip('#'));cell._tc.get_or_add_tcPr().append(shade)
                    p=cell.paragraphs[0];p.paragraph_format.space_before=Pt(9);p.paragraph_format.space_after=Pt(9)
                    r=p.add_run((label if index==0 else value.strip()) if sep else (line if index==1 else 'Contact'))
                    r.font.name=design['font'];r.font.size=Pt(10 if index==0 else 11);r.bold=index==0;r.font.color.rgb=RGBColor.from_string(design['primary' if index==0 else 'text'].lstrip('#'))
        end=paragraph('',1,after=0);end.paragraph_format.keep_with_next=False
        end.add_run().add_break(WD_BREAK.PAGE)
        prefix=[deepcopy(node) for node in cover.element.body if node.tag!='{'+W+'}sectPr']
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
