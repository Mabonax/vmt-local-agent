"""Trusted Office generation tools; return bytes, never accept output paths."""
import base64
import io
import json
import sys
import math

def string(v,limit):
    if type(v) is not str or len(v)>limit or any(ord(c)<32 and c not in "\n\r\t" for c in v): raise ValueError("Invalid text")
    return v

def items(v,limit):
    if type(v) is not list or len(v)>limit: raise ValueError("Invalid array")
    return v

def sections(v,limit,body_limit):
    for s in items(v,limit):
        if type(s) is not dict or set(s)!={"heading","body"}: raise ValueError()
        yield string(s["heading"],150),string(s["body"],body_limit)

def execute(name,args):
    if type(args) is not dict: raise ValueError()
    out=io.BytesIO()
    if name=="office.docx":
        from docx import Document
        from docx.shared import Inches, Pt, RGBColor
        if set(args)!={"title","sections"}: raise ValueError()
        doc=Document()
        page=doc.sections[0]
        page.top_margin=page.bottom_margin=Inches(.85)
        page.left_margin=page.right_margin=Inches(.9)
        normal=doc.styles["Normal"]
        normal.font.name="Calibri"; normal.font.size=Pt(11)
        normal.paragraph_format.space_after=Pt(7)
        normal.paragraph_format.line_spacing=1.15
        for style in ("Title","Heading 1"):
            doc.styles[style].font.color.rgb=RGBColor.from_string("17365D")
        doc.add_heading(string(args["title"],150),0)
        for heading,body in sections(args["sections"],30,4000):
            doc.add_heading(heading,1)
            for p in body.splitlines():
                if p.strip(): doc.add_paragraph(p)
        doc.save(out)
        extension="docx"
    elif name=="office.xlsx":
        from openpyxl import Workbook
        from openpyxl.styles import Font, PatternFill, Alignment
        from openpyxl.utils import get_column_letter
        if set(args)!={"headers","rows"}: raise ValueError()
        headers=[string(h,100) for h in items(args["headers"],30)]
        if not headers or any(not h.strip() for h in headers) or len(set(headers))!=len(headers): raise ValueError()
        wb=Workbook(); ws=wb.active; ws.title="Data"
        def write_row(row,i):
            if type(row) is not list or len(row)!=len(headers): raise ValueError()
            for j,v in enumerate(row,1):
                if type(v) is str:
                    string(v,500); c=ws.cell(i,j,v); c.data_type="s"
                elif v is None or type(v) is bool or (type(v) in (int,float) and math.isfinite(v)):
                    ws.cell(i,j,v)
                else: raise ValueError()
        write_row(headers,1)
        for i,row in enumerate(items(args["rows"],1000),2): write_row(row,i)
        for c in ws[1]:
            c.font=Font(bold=True,color="FFFFFF")
            c.fill=PatternFill("solid",fgColor="17365D")
            c.alignment=Alignment(wrap_text=True)
        for j,h in enumerate(headers,1):
            ws.column_dimensions[get_column_letter(j)].width=min(40,max(14,len(h)+4))
        ws.freeze_panes="A2"; ws.auto_filter.ref=ws.dimensions
        wb.save(out); extension="xlsx"
    elif name=="office.pptx":
        from pptx import Presentation
        from pptx.util import Inches, Pt
        from pptx.dml.color import RGBColor
        if set(args)!={"title","slides"}: raise ValueError()
        prs=Presentation(); prs.slide_width=Inches(13.333); prs.slide_height=Inches(7.5)
        def textbox(slide,x,y,w,h,value,size,bold=False):
            tf=slide.shapes.add_textbox(Inches(x),Inches(y),Inches(w),Inches(h)).text_frame
            tf.word_wrap=True
            tf.text=value
            for p in tf.paragraphs:
                p.font.name="Calibri"; p.font.size=Pt(size); p.font.bold=bold
                p.font.color.rgb=RGBColor.from_string("17365D" if bold else "253746")
                p.space_after=Pt(10)
        slide=prs.slides.add_slide(prs.slide_layouts[6])
        title=string(args["title"],100)
        textbox(slide,.8,2,11.7,3,title,40,True)
        for heading,body in sections(args["slides"],20,400):
            if len(heading)>80 or len(body.splitlines())>6: raise ValueError("Slide content limit")
            slide=prs.slides.add_slide(prs.slide_layouts[6])
            textbox(slide,.8,.6,11.7,1.4,heading,30,True)
            textbox(slide,.8,2.1,11.7,4.7,body,22)
        prs.save(out); extension="pptx"
    else: raise ValueError()
    data=out.getvalue()
    if len(data)>700000: raise ValueError("Artifact exceeds transport limit")
    return {"extension":extension,"encoding":"base64","bytes":len(data),"content":base64.b64encode(data).decode("ascii")}

def main():
    try:
        raw=sys.stdin.buffer.read(1048577)
        if len(raw)>1048576: raise ValueError()
        req=json.loads(raw)
        if type(req) is not dict or set(req)!={"tool","args"}: raise ValueError()
        print(json.dumps({"ok":True,"result":execute(req["tool"],req["args"])},allow_nan=False))
        return 0
    except (ValueError,TypeError,KeyError,ArithmeticError,RecursionError,ImportError):
        print('{"ok":false,"error":"Invalid input, unavailable dependency or artifact limit"}')
        return 1
if __name__=="__main__": sys.exit(main())
