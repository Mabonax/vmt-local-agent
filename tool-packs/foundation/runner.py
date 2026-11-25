"""Dependency-free, pure computation tools. No filesystem, network or subprocess access."""
import json
import math
import re
import sys
from collections import Counter

def number(x):
    if type(x) not in (int, float) or not math.isfinite(x):
        raise ValueError("Expected finite number")
    return x

def cents(value):
    # Canonical decimal strings only; no locale guessing or floating point.
    if type(value) is not str or len(value)>32 or not re.fullmatch(r"-?(?:0|[1-9][0-9]{0,14})(?:\.[0-9]{1,2})?",value):
        raise ValueError("Expected signed decimal amount with at most two fractional digits")
    negative=value.startswith("-")
    whole,_,fraction=value.lstrip("-").partition(".")
    result=int(whole)*100+int(fraction.ljust(2,"0"))
    return -result if negative else result

def monetary(value):
    return ("-" if value<0 else "")+str(abs(value)//100)+"."+str(abs(value)%100).zfill(2)

def execute(name, args):
    if not isinstance(args, dict):
        raise ValueError("Expected object")
    if name == "finance.check_total":
        if set(args)!={"currency","amounts","claimed_total"}: raise ValueError("Unexpected arguments")
        if args["currency"] not in ("ZAR","USD","EUR","GBP"): raise ValueError("Unsupported currency")
        amounts=args["amounts"]
        if type(amounts) is not list or not 1<=len(amounts)<=100: raise ValueError("Invalid line count")
        total=sum(map(cents,amounts))
        claimed=cents(args["claimed_total"])
        return {"currency":args["currency"],"line_count":len(amounts),"computed_total":monetary(total),"claimed_total":monetary(claimed),"difference":monetary(total-claimed),"matches":total==claimed,"limitation":"Arithmetic only; input amounts and currency are not independently verified"}
    if name == "math.calculate":
        if set(args) != {"operation", "a", "b"}:
            raise ValueError("Unexpected arguments")
        a, b = number(args["a"]), number(args["b"])
        op = args["operation"]
        if op == "add": result = a + b
        elif op == "subtract": result = a - b
        elif op == "multiply": result = a * b
        elif op == "divide": result = a / b
        else: raise ValueError("Unsupported operation")
        return {"value": number(result)}
    if name == "data.describe":
        if set(args) != {"values"}: raise ValueError("Unexpected arguments")
        v = args["values"]
        if not isinstance(v, list) or not 1 <= len(v) <= 10000:
            raise ValueError("Expected 1 to 10000 numbers")
        v = sorted(map(number, v))
        n = len(v)
        return {"count": n, "min": v[0], "max": v[-1],
                "mean": number(math.fsum(v) / n),
                "median": number(v[n//2] if n%2 else v[n//2-1]/2+v[n//2]/2)}
    if name == "text.inspect":
        if set(args) != {"text"}: raise ValueError("Unexpected arguments")
        t = args["text"]
        if not isinstance(t, str) or len(t)>100000: raise ValueError("Invalid text")
        return {"characters":len(t),"words":len(t.split()),"lines":len(t.splitlines())}
    if name == "data.frequency":
        if set(args) != {"values"}: raise ValueError("Unexpected arguments")
        v=args["values"]
        if not isinstance(v,list) or len(v)>1000 or any(not isinstance(x,str) or len(x)>200 for x in v):
            raise ValueError("Expected up to 1000 short strings")
        return {"counts":dict(Counter(v))}
    if name == "data.json_validate":
        if set(args) != {"text"}: raise ValueError("Unexpected arguments")
        t=args["text"]
        if not isinstance(t,str) or len(t)>100000: raise ValueError("Invalid text")
        def reject(s): raise ValueError("Nonstandard JSON constant")
        def unique(pairs):
            d={}
            for k,v in pairs:
                if k in d: raise ValueError("Duplicate key")
                d[k]=v
            return d
        json.loads(t,parse_constant=reject,object_pairs_hook=unique)
        return {"valid":True}
    raise ValueError("Unknown tool")

def main():
    try:
        raw=sys.stdin.buffer.read(1048577)
        if len(raw)>1048576: raise ValueError("Request too large")
        req=json.loads(raw)
        if not isinstance(req,dict) or set(req)!={"tool","args"}: raise ValueError("Invalid request")
        response={"ok":True,"result":execute(req["tool"],req["args"])}
        print(json.dumps(response,allow_nan=False))
        return 0
    except (ValueError,TypeError,ArithmeticError,RecursionError,KeyError):
        print(json.dumps({"ok":False,"error":"Invalid input or arithmetic failure"}))
        return 1

if __name__=="__main__":
    sys.exit(main())
