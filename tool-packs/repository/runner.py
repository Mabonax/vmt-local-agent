"""Static repository intelligence. Never imports or executes inspected code."""
import ast
import collections
import json
import pathlib
import sys
import tomllib

def execute(name,args):
    fields={"project.detect":{"paths"},"code.metrics":{"text"},"code.dependencies":{"format","text"},"python.imports":{"text"}}
    if name not in fields or type(args) is not dict or set(args)!=fields[name]: raise ValueError()
    if name=="project.detect":
        paths=args["paths"]
        if type(paths) is not list or len(paths)>2000 or any(type(p) is not str or len(p)>500 for p in paths): raise ValueError()
        names={pathlib.PurePosixPath(p.replace(chr(92),"/")).name for p in paths}
        markers={"Cargo.toml":"Rust","package.json":"Node.js","composer.json":"PHP","artisan":"Laravel","pubspec.yaml":"Dart/Flutter","pyproject.toml":"Python","requirements.txt":"Python","go.mod":"Go","Dockerfile":"Docker","pom.xml":"Java"}
        return {"detected":sorted({v for k,v in markers.items() if k in names}),"extensions":dict(collections.Counter(pathlib.PurePosixPath(p).suffix.lower() for p in paths if pathlib.PurePosixPath(p).suffix).most_common(30))}
    text=args["text"]
    if type(text) is not str or len(text)>100000: raise ValueError()
    if name=="code.metrics":
        lines=text.splitlines()
        return {"lines":len(lines),"nonblank_lines":sum(bool(x.strip()) for x in lines),"max_line_length":max(map(len,lines),default=0),"characters":len(text)}
    if name=="python.imports":
        tree=ast.parse(text)
        imports=[]
        for node in ast.walk(tree):
            if isinstance(node,ast.Import):
                imports.extend({"module":a.name,"line":node.lineno} for a in node.names)
            elif isinstance(node,ast.ImportFrom):
                imports.append({"module":"."*node.level+(node.module or ""),"line":node.lineno})
        return {"imports":imports[:500],"truncated":len(imports)>500}
    fmt=args["format"]
    if fmt in ("npm","composer"):
        obj=json.loads(text)
        groups=("dependencies","devDependencies","peerDependencies") if fmt=="npm" else ("require","require-dev")
    elif fmt=="cargo":
        obj=tomllib.loads(text)
        groups=("dependencies","dev-dependencies","build-dependencies")
    else: raise ValueError()
    if type(obj) is not dict: raise ValueError()
    result=[]
    for group in groups:
        entries=obj.get(group,{})
        if type(entries) is not dict: raise ValueError()
        for dep,value in sorted(entries.items()):
            if type(value) is str: version=value
            elif fmt=="cargo" and type(value) is dict: version=value.get("version","unspecified")
            else: raise ValueError()
            if type(version) is not str: raise ValueError()
            result.append({"name":dep[:200],"version":version[:200],"group":group})
    return {"dependencies":result[:500],"truncated":len(result)>500,"scope":"top-level declarations only"}

def main():
    try:
        raw=sys.stdin.buffer.read(1048577)
        if len(raw)>1048576: raise ValueError()
        req=json.loads(raw)
        if type(req) is not dict or set(req)!={"tool","args"}: raise ValueError()
        out=json.dumps({"ok":True,"result":execute(req["tool"],req["args"])},allow_nan=False)
        if len(out.encode())>1048575: raise ValueError()
        print(out)
        return 0
    except (ValueError,TypeError,KeyError,RecursionError,SyntaxError,OverflowError):
        print('{"ok":false,"error":"Invalid input"}')
        return 1
if __name__=="__main__": sys.exit(main())
