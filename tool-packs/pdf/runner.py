"""Bounded in-memory PDF tools. No output paths or external resource loading."""
import base64
import io
import json
import sys

MAX_RAW=500000

def artifact(data):
    if len(data)>MAX_RAW: raise ValueError("Artifact too large")
    return {"extension":"pdf","encoding":"base64","bytes":len(data),"content":base64.b64encode(data).decode("ascii")}

def reader(encoded):
    from pypdf import PdfReader
    if type(encoded) is not str or len(encoded)>670000: raise ValueError()
    data=base64.b64decode(encoded,validate=True)
    if len(data)>MAX_RAW or not data.startswith(b"%PDF-"): raise ValueError()
    r=PdfReader(io.BytesIO(data),strict=True)
    if r.is_encrypted or not 1<=len(r.pages)<=50: raise ValueError("Unsupported PDF")
    return r

def page_range(pages,n):
    if type(pages) is not list or not 1<=len(pages)<=50 or any(type(p) is not int or not 1<=p<=n for p in pages): raise ValueError("Invalid page numbers")
    return pages

def execute(name,args):
    if type(args) is not dict: raise ValueError()
    fields={"pdf.generate":{"title","body"},"pdf.inspect":{"content"},"pdf.extract_text":{"content","pages"},"pdf.split":{"content","pages"},"pdf.merge":{"documents"}}
    if name not in fields or set(args)!=fields[name]: raise ValueError()
    if name=="pdf.generate":
        from reportlab.pdfgen import canvas
        from reportlab.lib.pagesizes import A4
        from reportlab.pdfbase.pdfmetrics import stringWidth
        title,body=args["title"],args["body"]
        if type(title) is not str or not 1<=len(title)<=100 or type(body) is not str or len(body)>30000: raise ValueError()
        # Built-in font support is deliberately limited to Latin-1 text.
        for value in (title,body):
            value.encode("latin-1")
            if any(ord(c)<32 and c not in "\n\r\t" for c in value): raise ValueError()
        if "\n" in title or "\r" in title or "\t" in title: raise ValueError()
        out=io.BytesIO(); c=canvas.Canvas(out,pagesize=A4)
        w,h=A4; page=1
        def wrap(text,font,size):
            # Width-aware wrapping, including long unbroken words.
            lines=[]; line=""
            for word in text.split():
                candidate=(line+" "+word).strip()
                if stringWidth(candidate,font,size)<=w-96:
                    line=candidate; continue
                if line: lines.append(line); line=""
                for ch in word:
                    if stringWidth(line+ch,font,size)>w-96:
                        lines.append(line); line=""
                    line+=ch
            if line: lines.append(line)
            return lines or [""]
        def header():
            c.setFont("Helvetica-Bold",16); c.setFillColorRGB(.09,.21,.36)
            for i,line in enumerate(wrap(title,"Helvetica-Bold",16)): c.drawString(48,h-48-20*i,line)
            c.setFont("Helvetica",10); c.setFillColorRGB(.14,.18,.23)
        def footer():
            c.setFont("Helvetica",9); c.drawRightString(w-48,32,str(page))
        header(); y=h-110
        for paragraph in body.splitlines():
            for line in wrap(paragraph.expandtabs(4),"Helvetica",10):
                if y<60:
                    footer(); c.showPage(); page+=1
                    if page>50: raise ValueError()
                    header(); y=h-110
                c.drawString(48,y,line); y-=15
        footer(); c.save()
        return artifact(out.getvalue())
    from pypdf import PdfWriter
    if name=="pdf.merge":
        docs=args["documents"]
        if type(docs) is not list or not 1<=len(docs)<=5: raise ValueError()
        writer=PdfWriter()
        for d in docs:
            r=reader(d)
            if len(writer.pages)+len(r.pages)>50: raise ValueError()
            for p in r.pages: writer.add_page(p,excluded_keys=("/Annots","/AA","/B"))
        out=io.BytesIO(); writer.write(out); return artifact(out.getvalue())
    r=reader(args["content"])
    if name=="pdf.inspect":
        return {"page_count":len(r.pages),"pages":[{"number":i,"width_pt":float(p.mediabox.width),"height_pt":float(p.mediabox.height),"rotation":p.rotation} for i,p in enumerate(r.pages,1)]}
    pages=page_range(args["pages"],len(r.pages))
    if name=="pdf.extract_text":
        result=[]; remaining=30000
        for n in pages:
            t=r.pages[n-1].extract_text() or ""
            result.append({"page":n,"text":t[:remaining],"truncated":len(t)>remaining})
            remaining-=min(remaining,len(t))
        return {"pages":result,"ocr":False}
    writer=PdfWriter()
    for n in pages: writer.add_page(r.pages[n-1],excluded_keys=("/Annots","/AA","/B"))
    out=io.BytesIO(); writer.write(out); return artifact(out.getvalue())

def main():
    try:
        raw=sys.stdin.buffer.read(1048577)
        if len(raw)>1048576: raise ValueError()
        req=json.loads(raw)
        if type(req) is not dict or set(req)!={"tool","args"}: raise ValueError()
        output=json.dumps({"ok":True,"result":execute(req["tool"],req["args"])},allow_nan=False)
        if len(output.encode())>1048575: raise ValueError()
        print(output); return 0
    except Exception:
        print('{"ok":false,"error":"Invalid PDF request, unsupported content or resource limit"}')
        return 1
if __name__=="__main__": sys.exit(main())
