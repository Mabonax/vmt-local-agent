"""Read-only host diagnostics. No shell, arbitrary paths, process lists or identifiers."""
import importlib.util
import math
import os
import platform
import shutil
import struct
import sys
from pathlib import Path

MODULES=("docx","openpyxl","pptx","pypdf","reportlab","pypdfium2","PIL","psutil")
def obj(props):
    return {"type":"object","properties":props,"required":list(props),"additionalProperties":False}
def integer(a,b):
    return {"type":"integer","minimum":a,"maximum":b}
SCHEMAS={
 "system.info":obj({}),
 "system.cpu":obj({}),
 "system.memory":obj({}),
 "system.storage":obj({}),
 "system.dependencies":obj({}),
 "model.compatibility":obj({"parameters_million":integer(1,1000000),"bits_per_weight":integer(1,32),"context_tokens":integer(128,1000000),"kv_bytes_per_token":integer(0,10000000),"runtime_overhead_mib":integer(0,1000000),"available_mib":integer(0,1000000)})
}
def estimate(args):
    if type(args) is not dict or set(args)!=set(SCHEMAS["model.compatibility"]["properties"]): raise ValueError("Invalid estimate")
    for key,schema in SCHEMAS["model.compatibility"]["properties"].items():
        v=args[key]
        if type(v) is not int or not schema["minimum"]<=v<=schema["maximum"]: raise ValueError("Invalid estimate")
    weights=math.ceil(args["parameters_million"]*1000000*args["bits_per_weight"]/8/1048576)
    kv=math.ceil(args["context_tokens"]*args["kv_bytes_per_token"]/1048576)
    total=weights+kv+args["runtime_overhead_mib"]
    return {"weights_mib":weights,"kv_cache_mib":kv,"estimated_total_mib":total,"headroom_mib":args["available_mib"]-total,"fits_estimate":total<=args["available_mib"],"assumption":"Single memory pool; caller-supplied effective bits and KV cost; excludes unmodelled allocations"}

class SystemHost:
    def __init__(self,delegate,workspace):
        self._delegate=delegate
        self._workspace=Path(workspace).resolve(strict=True)
        if not self._workspace.is_dir(): raise ValueError("Invalid workspace")
    def call(self,name,args,previous=None):
        if name not in SCHEMAS: return self._delegate.call(name,args,previous=previous)
        if previous is not None or type(args) is not dict: raise ValueError("Invalid request")
        if name=="model.compatibility": result=estimate(args)
        else:
            if args: raise ValueError("No arguments accepted")
            if name=="system.info":
                result={"os":platform.system(),"release":platform.release(),"architecture":platform.machine(),"python":platform.python_version(),"pointer_bits":struct.calcsize("P")*8}
            elif name=="system.cpu":
                result={"logical_cores":os.cpu_count(),"architecture":platform.machine()}
            elif name=="system.storage":
                d=shutil.disk_usage(self._workspace)
                result={"total_bytes":d.total,"used_bytes":d.used,"free_bytes":d.free,"scope":"workspace volume"}
            elif name=="system.memory":
                try:
                    import psutil
                    m=psutil.virtual_memory()
                    result={"available":True,"total_bytes":m.total,"available_bytes":m.available}
                except (ImportError,OSError,NotImplementedError):
                    result={"available":False,"reason":"Memory provider unavailable"}
            else:
                result={"modules":{m:importlib.util.find_spec(m) is not None for m in MODULES},"check":"discoverability only"}
        return {"ok":True,"result":result}
