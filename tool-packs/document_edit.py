"""Conservative exact replacement in simple DOCX body text runs."""
import hashlib
import io
import zipfile
from copy import copy
from lxml import etree as ET

W="{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
XML_SPACE="{http://www.w3.org/XML/1998/namespace}space"
PART="word/document.xml"

def parse(raw):
    return ET.fromstring(raw,ET.XMLParser(resolve_entities=False,no_network=True))

def replace_text(source,old,new,expected):
    if not 0<len(source)<=700000 or type(expected) is not int or not 1<=expected<=100:
        raise ValueError("Invalid edit bounds")
    if not isinstance(old,str) or not isinstance(new,str) or not 1<=len(old)<=500 or len(new)>500 or old==new:
        raise ValueError("Invalid replacement")
    if any(ord(c)<32 for c in old+new):
        raise ValueError("Only single-line text is supported")
    with zipfile.ZipFile(io.BytesIO(source)) as archive:
        entries=archive.infolist()
        names=archive.namelist()
        if len(entries)>2000 or len(set(names))!=len(names) or sum(x.file_size for x in entries)>64*1024*1024:
            raise ValueError("Invalid package bounds")
        if any(n.lower().startswith("_xmlsignatures/") for n in names):
            raise ValueError("Signed documents are unsupported")
        tree=parse(archive.read(PART))
        body=tree.find(W+"body")
        if body is None: raise ValueError("Missing document body")
        changes=[]
        for p in body.iter(W+"p"):
            # Reject matches in fields, revisions, nested text boxes or other
            # complex content; do not attempt cross-run reconstruction.
            nodes=list(p.iter(W+"t"))
            joined="".join(n.text or "" for n in nodes)
            count=joined.count(old)
            if not count: continue
            simple=all(n.getparent().tag==W+"r" and n.getparent().getparent() is p for n in nodes)
            complex_tags={W+x for x in ("fldChar","instrText","fldSimple","ins","del","drawing","pict","sdt","hyperlink","tab","br")}
            if not simple or any(n.tag in complex_tags for n in p.iter()) or any(a.tag in complex_tags for a in p.iterancestors()):
                raise ValueError("Match occurs in unsupported complex paragraph")
            if sum((n.text or "").count(old) for n in nodes)!=count:
                raise ValueError("Match crosses text runs")
            for n in nodes:
                if old in (n.text or ""):
                    changes.append((n,n.text,n.text.replace(old,new),n.get(XML_SPACE)))
        actual=sum(before.count(old) for _,before,_,_ in changes)
        if actual!=expected: raise ValueError("Expected match count differs")
        original=ET.tostring(tree,method="c14n")
        for node,before,after,space in changes:
            node.text=after
            if after!=after.strip(): node.set(XML_SPACE,"preserve")
        replacement=ET.tostring(tree,encoding="UTF-8",xml_declaration=True,standalone=True)
        expected_xml=ET.tostring(tree,method="c14n")
        # Reverse only the authorised nodes and confirm the original tree.
        for node,before,after,space in changes:
            node.text=before
            if space is None: node.attrib.pop(XML_SPACE,None)
            else: node.set(XML_SPACE,space)
        if ET.tostring(tree,method="c14n")!=original:
            raise ValueError("Unexpected structure change")
        output=io.BytesIO()
        with zipfile.ZipFile(output,"w") as result:
            for item in entries:
                result.writestr(copy(item),replacement if item.filename==PART else archive.read(item))
        raw=output.getvalue()
        if len(raw)>700000: raise ValueError("Edited artifact exceeds limit")
        with zipfile.ZipFile(io.BytesIO(raw)) as result:
            if result.namelist()!=names or any(result.read(n)!=archive.read(n) for n in names if n!=PART):
                raise ValueError("Unexpected package change")
            if ET.tostring(parse(result.read(PART)),method="c14n")!=expected_xml:
                raise ValueError("Edited XML read-back differs")
    from docx import Document
    Document(io.BytesIO(raw))
    return raw,{"source_sha256":hashlib.sha256(source).hexdigest(),"replacements":actual,
                "unchanged_package_parts":True,"authorised_text_only":True,"visual_review":"PENDING",
                "scope":"Simple body paragraph text runs, including table paragraphs; headers and other parts untouched"}
