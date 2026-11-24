"""Standalone dispatcher for explicitly trusted, pure tool packs only.

Not a sandbox. Never use with downloaded or model-authored executable code.
Workspace and network operations belong to separate policy-controlled adapters.
"""
import argparse
import json
import math
import pathlib
import re
import subprocess
import sys

ROOT=pathlib.Path(__file__).resolve().parent
TRUSTED=("foundation","analysis","documents","repository","office")
LIMIT=1048576

def validate(value, schema):
    t=schema["type"]
    if t=="object":
        if type(value) is not dict: raise ValueError("Expected object")
        props=schema["properties"]
        if not set(schema.get("required",[]))<=set(value) or set(value)-set(props): raise ValueError("Invalid object fields")
        for k,v in value.items(): validate(v,props[k])
    elif t=="array":
        if type(value) is not list or not schema.get("minItems",0)<=len(value)<=schema.get("maxItems",10000): raise ValueError("Invalid array")
        for v in value: validate(v,schema["items"])
    elif t=="string":
        if type(value) is not str or not schema.get("minLength",0)<=len(value)<=schema.get("maxLength",100000): raise ValueError("Invalid string")
        if "pattern" in schema and not re.fullmatch(schema["pattern"],value): raise ValueError("Invalid pattern")
    elif t=="scalar":
        if value is not None and type(value) not in (str,bool,int,float): raise ValueError("Invalid scalar")
        if type(value) is str and len(value)>500: raise ValueError("Invalid string")
        if type(value) in (int,float) and not math.isfinite(value): raise ValueError("Invalid number")
    elif t=="number":
        if type(value) not in (int,float) or not math.isfinite(value): raise ValueError("Invalid number")
    else: raise ValueError("Unsupported schema")
    if "enum" in schema and value not in schema["enum"]: raise ValueError("Invalid choice")

def registry():
    result={}
    for name in TRUSTED:
        folder=ROOT/name
        if folder.is_symlink() or (folder/"manifest.json").is_symlink() or (folder/"runner.py").is_symlink(): raise ValueError("Linked pack rejected")
        m=json.loads((folder/"manifest.json").read_text(encoding="utf-8"))
        if m["runtime"]!="python3" or m["protocol"]!="stdin-json-v1" or m["entrypoint"]!="runner.py" or m["permissions"] or m["network"] is not False: raise ValueError("Unsupported pack policy")
        if not 1<=m["timeout_seconds"]<=10 or not 1<=m["max_input_bytes"]<=LIMIT or not 1<=m["max_output_bytes"]<=LIMIT: raise ValueError("Invalid limits")
        for tool in m["tools"]:
            if tool["name"] in result: raise ValueError("Duplicate tool")
            result[tool["name"]]=(folder,m,tool)
    return result

def dispatch(name,args):
    folder,m,tool=registry()[name]
    validate(args,tool["input_schema"])
    request=json.dumps({"tool":name,"args":args},allow_nan=False).encode()
    if len(request)>m["max_input_bytes"]: raise ValueError("Request too large")
    # Trusted pure runners have bounded inputs; communicate buffers their output.
    # This is a post-execution output check, not a hard process memory limit.
    p=subprocess.run([sys.executable,"-I","-B",str(folder/"runner.py")],input=request,capture_output=True,timeout=m["timeout_seconds"],cwd=folder,env={})
    if len(p.stdout)>m["max_output_bytes"] or len(p.stderr)>LIMIT: raise ValueError("Output too large")
    response=json.loads(p.stdout)
    if not isinstance(response,dict) or type(response.get("ok")) is not bool or p.returncode!= (0 if response["ok"] else 1): raise ValueError("Invalid response")
    return response

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("action",choices=["list","call"])
    parser.add_argument("name",nargs="?")
    a=parser.parse_args()
    try:
        if a.action=="list":
            response={"ok":True,"tools":[v[2] for v in registry().values()]}
        else:
            raw=sys.stdin.buffer.read(LIMIT+1)
            if len(raw)>LIMIT: raise ValueError("Request too large")
            response=dispatch(a.name,json.loads(raw))
        print(json.dumps(response,allow_nan=False))
        return 0 if response["ok"] else 1
    except (ValueError,KeyError,TypeError,ArithmeticError,RecursionError,OSError,subprocess.TimeoutExpired):
        print('{"ok":false,"error":"Tool request rejected or execution failed"}')
        return 1

if __name__=="__main__": sys.exit(main())
