"""Pure text and data analysis; no external I/O beyond the JSON protocol."""
import csv
import difflib
import hashlib
import io
import json
import sys
import ast
import collections

def execute(name, args):
    if not isinstance(args,dict): raise ValueError("Expected object")
    expected={"text.sha256":{"text"}, "text.diff":{"before","after"}, "csv.profile":{"text"}, "python.symbols":{"text"}, "text.find":{"text","query"}}
    if name not in expected or set(args)!=expected[name]: raise ValueError("Invalid arguments")
    if any(not isinstance(v,str) or len(v)>100000 for v in args.values()): raise ValueError("Invalid text")
    if name=="text.sha256":
        return {"sha256":hashlib.sha256(args["text"].encode()).hexdigest()}
    if name=="text.diff":
        lines=list(difflib.unified_diff(args["before"].splitlines(),args["after"].splitlines(),fromfile="before",tofile="after",lineterm=""))
        return {"lines":lines[:200],"truncated":len(lines)>200}
    if name=="csv.profile":
        reader=csv.reader(io.StringIO(args["text"]),strict=True)
        header=next(reader,[])
        if not header or len(header)>100 or len(set(header))!=len(header): raise ValueError("Invalid header")
        counts=[0]*len(header); rows=0
        for row in reader:
            if len(row)!=len(header) or rows>=10000: raise ValueError("Invalid row")
            rows+=1
            for i,v in enumerate(row): counts[i]+=not bool(v.strip())
        return {"rows":rows,"columns":[{"name":h,"missing":n} for h,n in zip(header,counts)]}
    if name=="python.symbols":
        tree=ast.parse(args["text"])
        nodes=[{"name":n.name,"kind":type(n).__name__,"line":n.lineno} for n in ast.walk(tree) if isinstance(n,(ast.FunctionDef,ast.AsyncFunctionDef,ast.ClassDef))]
        return {"symbols":nodes[:500],"truncated":len(nodes)>500}
    q=args["query"]
    if not q or len(q)>300: raise ValueError("Invalid query")
    matches=[{"line":i,"text":line[:500]} for i,line in enumerate(args["text"].splitlines(),1) if q.casefold() in line.casefold()]
    return {"matches":matches[:100],"truncated":len(matches)>100}

def main():
    try:
        raw=sys.stdin.buffer.read(1048577)
        if len(raw)>1048576: raise ValueError()
        req=json.loads(raw)
        if not isinstance(req,dict) or set(req)!={"tool","args"}: raise ValueError()
        print(json.dumps({"ok":True,"result":execute(req["tool"],req["args"])},allow_nan=False))
        return 0
    except (ValueError,TypeError,KeyError,RecursionError,SyntaxError,csv.Error):
        print('{"ok":false,"error":"Invalid input"}')
        return 1
if __name__=="__main__": sys.exit(main())
