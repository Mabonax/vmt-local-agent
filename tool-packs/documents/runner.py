"""Dependency-free document planning and validation, not binary Office rendering."""
import json
import sys
import html
import re

def execute(name,args):
    if not isinstance(args,dict): raise ValueError()
    if name=="document.outline":
        if set(args)!={"text"} or not isinstance(args["text"],str) or len(args["text"])>100000: raise ValueError()
        headings=[]; previous=0; warnings=[]
        for i,line in enumerate(args["text"].splitlines(),1):
            m=re.match(r"^(#{1,6}) +(.+)$",line)
            if m:
                level=len(m[1])
                if level>previous+1: warnings.append({"line":i,"issue":"Skipped heading level"})
                headings.append({"level":level,"title":m[2],"line":i}); previous=level
        return {"headings":headings[:200],"warnings":warnings[:200],"truncated":len(headings)>200}
    if name=="design.contrast":
        if set(args)!={"foreground","background"}: raise ValueError()
        def luminance(s):
            if not isinstance(s,str) or not re.fullmatch(r"#[0-9a-fA-F]{6}",s): raise ValueError()
            rgb=[int(s[i:i+2],16)/255 for i in (1,3,5)]
            c=[v/12.92 if v<=0.04045 else ((v+0.055)/1.055)**2.4 for v in rgb]
            return sum(a*b for a,b in zip(c,(0.2126,0.7152,0.0722)))
        a,b=sorted([luminance(args["foreground"]),luminance(args["background"])])
        ratio=(b+0.05)/(a+0.05)
        return {"ratio":round(ratio,4),"aa_normal":ratio>=4.5,"aa_large":ratio>=3,"aaa_normal":ratio>=7}
    if name=="document.html":
        if set(args)!={"title","sections"}: raise ValueError()
        title=args["title"]; sections=args["sections"]
        if not isinstance(title,str) or not 1<=len(title)<=200 or not isinstance(sections,list) or len(sections)>50: raise ValueError()
        parts=[]
        for s in sections:
            if not isinstance(s,dict) or set(s)!={"heading","body"} or any(not isinstance(v,str) or len(v)>5000 for v in s.values()): raise ValueError()
            parts.append("<section><h2>"+html.escape(s["heading"])+"</h2>"+"".join("<p>"+html.escape(p)+"</p>" for p in s["body"].splitlines() if p.strip())+"</section>")
        markup='<!doctype html><html lang="en"><meta charset="utf-8"><title>'+html.escape(title)+'</title><style>body{font:16px/1.6 system-ui,sans-serif;max-width:720px;margin:40px auto;padding:0 24px;color:#182433}h1,h2{line-height:1.2}h2{margin-top:1.8em}p{margin:0 0 1em}@media print{body{margin:0;max-width:none}h2{break-after:avoid}}</style><main><h1>'+html.escape(title)+'</h1>'+"".join(parts)+"</main></html>"
        if len(markup.encode())>500000: raise ValueError()
        return {"html":markup}
    raise ValueError("Unknown tool")

def main():
    try:
        raw=sys.stdin.buffer.read(1048577)
        if len(raw)>1048576: raise ValueError()
        req=json.loads(raw)
        if not isinstance(req,dict) or set(req)!={"tool","args"}: raise ValueError()
        output=json.dumps({"ok":True,"result":execute(req["tool"],req["args"])},ensure_ascii=False,allow_nan=False)
        if len(output.encode("utf-8"))>1048576: raise ValueError()
        sys.stdout.buffer.write((output+"\n").encode("utf-8"))
        return 0
    except (ValueError,TypeError,KeyError,RecursionError):
        print('{"ok":false,"error":"Invalid input"}')
        return 1
if __name__=="__main__": sys.exit(main())
