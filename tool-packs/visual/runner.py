"""Bounded PDF previews and raster diagnostics; no model-controlled paths."""
import base64
import io
import json
import math
import sys
import warnings
from PIL import Image, ImageChops, ImageStat

MAX_PIXELS=2000000

def raw(value):
    if type(value) is not str or len(value)>940000: raise ValueError()
    data=base64.b64decode(value,validate=True)
    if not 0<len(data)<=700000: raise ValueError()
    return data

def image(value):
    with warnings.catch_warnings():
        warnings.simplefilter("error",Image.DecompressionBombWarning)
        with Image.open(io.BytesIO(raw(value)),formats=["PNG","JPEG"]) as im:
            if im.width*im.height>MAX_PIXELS or getattr(im,"n_frames",1)!=1: raise ValueError()
            im.load()
            rgba=im.convert("RGBA")
            bg=Image.new("RGBA",im.size,"white")
            bg.alpha_composite(rgba)
            return bg.convert("RGB")

def artifact(im):
    out=io.BytesIO()
    im.save(out,format="PNG")
    data=out.getvalue()
    if len(data)>700000: raise ValueError("Preview exceeds transport limit")
    return {"extension":"png","encoding":"base64","bytes":len(data),"content":base64.b64encode(data).decode("ascii")}

def execute(name,args):
    fields={"pdf.render":{"content","page","dpi"},"image.inspect":{"content"},"image.compare":{"first","second"}}
    if name not in fields or type(args) is not dict or set(args)!=fields[name]: raise ValueError()
    if name=="pdf.render":
        import pypdfium2 as pdfium
        data=raw(args["content"])
        if not data.startswith(b"%PDF-"): raise ValueError()
        page,dpi=args["page"],args["dpi"]
        if type(page) is not int or type(dpi) is not int or not 1<=page<=50 or not 36<=dpi<=144: raise ValueError()
        doc=pdfium.PdfDocument(data)
        try:
            if not 1<=len(doc)<=50 or page>len(doc): raise ValueError()
            p=doc[page-1]
            try:
                w,h=p.get_size(); scale=dpi/72
                if not math.isfinite(w*h) or min(w,h)<=0 or math.ceil(w*scale)*math.ceil(h*scale)>MAX_PIXELS: raise ValueError()
                bitmap=p.render(scale=scale)
                try:
                    im=bitmap.to_pil().convert("RGB")
                    return artifact(im)
                finally: bitmap.close()
            finally: p.close()
        finally: doc.close()
    if name=="image.inspect":
        im=image(args["content"])
        gray=im.convert("L")
        histogram=gray.histogram()
        dark=sum(histogram[:245])
        mask=gray.point(lambda p:255 if p<245 else 0)
        return {"width":im.width,"height":im.height,"nonwhite_fraction":round(dark/(im.width*im.height),6),"content_bounds":mask.getbbox(),"mean_luminance":round(ImageStat.Stat(gray).mean[0],3)}
    a,b=image(args["first"]),image(args["second"])
    if a.size!=b.size: raise ValueError("Image dimensions differ")
    diff=ImageChops.difference(a,b)
    r,g,bl=diff.split()
    mask=ImageChops.lighter(ImageChops.lighter(r,g),bl)
    changed=sum(mask.histogram()[1:])
    return {"width":a.width,"height":a.height,"changed_fraction":round(changed/(a.width*a.height),6),"mean_absolute_channel_difference":round(sum(ImageStat.Stat(diff).mean)/3,4),"difference_bounds":diff.getbbox()}

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
        print('{"ok":false,"error":"Invalid preview request or resource limit"}')
        return 1
if __name__=="__main__": sys.exit(main())
