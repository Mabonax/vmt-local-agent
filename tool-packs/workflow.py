"""Bounded sequential workflows for trusted host-selected templates.
No arbitrary code, shell execution, external paths or automatic retries.
Cancellation and deadlines are checked between tool calls, not during them.
"""
import copy
import json
import math
import re
import time
import uuid

MAX_STEPS=20
MAX_JSON=1048576
ALLOWED={"pdf.generate","pdf.inspect","pdf.render","image.inspect","artifact.inspect",
         "artifact.hash","office.docx","office.xlsx","office.pptx","document.outline",
         "design.contrast","layout.bounds","layout.overlap","data.describe"}

def bounded(value):
    if len(json.dumps(value,allow_nan=False).encode())>MAX_JSON:
        raise ValueError("Workflow data exceeds limit")

def resolve(value,inputs,results,depth=0):
    if depth>16: raise ValueError("Mapping depth exceeded")
    if isinstance(value,dict):
        if set(value)=={"ref"}:
            ref=value["ref"]
            if not isinstance(ref,str) or len(ref)>200 or not re.fullmatch(r"(input|step)(\.[a-zA-Z0-9_]+)+",ref): raise ValueError("Invalid reference")
            parts=ref.split(".")
            current=inputs if parts[0]=="input" else results
            for key in parts[1:]:
                if not isinstance(current,dict) or key not in current: raise ValueError("Unresolved reference")
                current=current[key]
            return copy.deepcopy(current)
        return {k:resolve(v,inputs,results,depth+1) for k,v in value.items()}
    if isinstance(value,list): return [resolve(v,inputs,results,depth+1) for v in value]
    return value

def gate(value,op,target):
    if op=="eq": return type(value) is type(target) and value==target
    if op in ("gt","ge","lt","le"):
        if any(type(v) not in (int,float) or not math.isfinite(v) for v in (value,target)): raise ValueError("Numeric gate required")
        return {"gt":value>target,"ge":value>=target,"lt":value<target,"le":value<=target}[op]
    raise ValueError("Unsupported gate")

def validate_plan(plan):
    bounded(plan)
    if type(plan) is not dict or set(plan)!={"steps","outputs"}: raise ValueError("Invalid plan")
    steps=plan["steps"]
    if type(steps) is not list or not 1<=len(steps)<=MAX_STEPS or type(plan["outputs"]) is not dict: raise ValueError("Invalid workflow size")
    seen=set()
    for step in steps:
        if type(step) is not dict or not {"id","tool","args"}<=set(step) or set(step)-{"id","tool","args","gates"}: raise ValueError("Invalid step")
        sid=step["id"]
        if not isinstance(sid,str) or not re.fullmatch(r"[a-z][a-z0-9_]{0,39}",sid) or sid in seen: raise ValueError("Invalid or duplicate step")
        if step["tool"] not in ALLOWED or type(step["args"]) is not dict: raise ValueError("Tool not allowed")
        gates=step.get("gates",[])
        if type(gates) is not list or len(gates)>8: raise ValueError("Invalid gates")
        for g in gates:
            if type(g) is not dict or set(g)!={"value","op","target"} or g["op"] not in ("eq","gt","ge","lt","le"): raise ValueError("Invalid gate")
        seen.add(sid)

def run(host,plan,inputs,cancel=None,deadline_seconds=120):
    """Return compact outputs and a payload-free journal. Partial artifacts remain session-scoped."""
    if type(inputs) is not dict or type(deadline_seconds) not in (int,float) or not 0<deadline_seconds<=120: raise ValueError("Invalid workflow inputs")
    validate_plan(plan); bounded(inputs)
    plan=copy.deepcopy(plan); inputs=copy.deepcopy(inputs)
    results={}; journal=[]; started=time.monotonic()
    report={"run_id":uuid.uuid4().hex,"status":"running","journal":journal}
    def stopped():
        return (cancel is not None and cancel.is_set()) or time.monotonic()-started>=deadline_seconds
    for step in plan["steps"]:
        if stopped():
            report["status"]="cancelled" if cancel is not None and cancel.is_set() else "deadline"
            return report
        event={"step":step["id"],"tool":step["tool"],"status":"running"}
        journal.append(event)
        try:
            args=resolve(step["args"],inputs,results); bounded(args)
            response=host.call(step["tool"],args)
            if type(response) is not dict or response.get("ok") is not True or "result" not in response:
                event["status"]="tool_failed"; report["status"]="failed"; return report
            results[step["id"]]=response["result"]; bounded(results)
            for g in step.get("gates",[]):
                if not gate(resolve(g["value"],inputs,results),g["op"],resolve(g["target"],inputs,results)):
                    event["status"]="gate_failed"; report["status"]="failed"; return report
            event["status"]="passed"
        except Exception:
            event["status"]="error"; report["status"]="failed"; return report
    if stopped():
        report["status"]="cancelled" if cancel is not None and cancel.is_set() else "deadline"
        return report
    try:
        outputs=resolve(plan["outputs"],inputs,results); bounded(outputs)
    except Exception:
        report["status"]="failed"; return report
    report.update(status="passed",outputs=outputs)
    return report
