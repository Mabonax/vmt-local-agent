"""Deterministic artifact and geometry diagnostics; not visual rendering."""
import base64
import hashlib
import io
import json
import math
import sys
import zipfile

def raw(content):
    if type(content) is not str or len(content)>940000: raise ValueError()
    data=base64.b64decode(content,validate=True)
    if not data or len(data)>700000: raise ValueError()
    return data

def inspect(data):
    if data.startswith(b"%PDF-"):
        from pypdf import PdfReader
        p=PdfReader(io.BytesIO(data),strict=True)
        if p.is_encrypted or not 1<=len(p.pages)<=50: raise ValueError()
        return {"format":"pdf","pages":len(p.pages),"structural_check":"parser_open"}
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        entries=z.infolist()
        names={e.filename for e in entries}
        if len(entries)>1000 or len(names)!=len(entries) or sum(e.file_size for e in entries)>12000000: raise ValueError()
        if any(e.flag_bits&1 or e.file_size>4000000 or ".." in e.filename.split("/") or e.filename.startswith("/") or chr(92) in e.filename for e in entries): raise ValueError()
        if "[Content_Types].xml" not in names or "_rels/.rels" not in names: raise ValueError()
        types=[(f,p) for f,p in (("docx","word/document.xml"),("xlsx","xl/workbook.xml"),("pptx","ppt/presentation.xml")) if p in names]
        if len(types)!=1 or z.testzip() is not None: raise ValueError()
        return {"format":types[0][0],"entries":len(entries),"structural_check":"zip_crc_and_required_parts"}

def numeric(v):
    if type(v) not in (int,float) or not math.isfinite(v) or abs(v)>1000000: raise ValueError()
    return v

def geometry(args):
    w,h=numeric(args["width"]),numeric(args["height"])
    if w<=0 or h<=0: raise ValueError()
    items=args["elements"]
    if type(items) is not list or len(items)>200: raise ValueError()
    boxes=[]; ids=set()
    for e in items:
        if type(e) is not dict or set(e)!={"id","x","y","width","height"}: raise ValueError()
        i=e["id"]
        if type(i) is not str or not 1<=len(i)<=100 or i in ids: raise ValueError()
        ids.add(i)
        x,y,a,b=[numeric(e[k]) for k in ("x","y","width","height")]
        if a<=0 or b<=0: raise ValueError()
        boxes.append((i,x,y,a,b))
    return w,h,boxes

def execute(name,args):
    fields={"artifact.hash":{"content"},"artifact.inspect":{"content"},"layout.bounds":{"width","height","elements"},"layout.overlap":{"width","height","elements"}}
    if name not in fields or type(args) is not dict or set(args)!=fields[name]: raise ValueError()
    if name=="artifact.hash":
        data=raw(args["content"])
        return {"sha256":hashlib.sha256(data).hexdigest(),"bytes":len(data)}
    if name=="artifact.inspect":
        return inspect(raw(args["content"]))
    w,h,boxes=geometry(args)
    if name=="layout.bounds":
        issues=[i for i,x,y,a,b in boxes if x<0 or y<0 or x+a>w or y+b>h]
        return {"outside":issues,"count":len(issues)}
    issues=[]; count=0
    for n,(i,x,y,a,b) in enumerate(boxes):
        for j,u,v,c,d in boxes[n+1:]:
            area=max(0,min(x+a,u+c)-max(x,u))*max(0,min(y+b,v+d)-max(y,v))
            if area>0:
                count+=1
                if len(issues)<200: issues.append({"first":i,"second":j,"area":area})
    return {"overlaps":issues,"count":count,"truncated":count>len(issues)}

def main():
    try:
        data=sys.stdin.buffer.read(1048577)
        if len(data)>1048576: raise ValueError()
        req=json.loads(data)
        if type(req) is not dict or set(req)!={"tool","args"}: raise ValueError()
        out=json.dumps({"ok":True,"result":execute(req["tool"],req["args"])},allow_nan=False)
        if len(out.encode())>1048575: raise ValueError()
        print(out); return 0
    except Exception:
        print('{"ok":false,"error":"Invalid artifact or geometry request"}')
        return 1
if __name__=="__main__": sys.exit(main())
