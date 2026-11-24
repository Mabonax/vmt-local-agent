"""Deterministic discovery of host-callable tools; not an authorization boundary."""
import copy
import importlib.util
import json
import re
from dispatch import registry
from artifact_host import SINGLE
from system_adapter import SCHEMAS

DEPENDENCIES={
 "office.docx":("docx",), "office.xlsx":("openpyxl",), "office.pptx":("pptx",),
 "pdf.generate":("reportlab",), "pdf.inspect":("pypdf",), "pdf.extract_text":("pypdf",),
 "pdf.split":("pypdf",), "pdf.merge":("pypdf",), "artifact.inspect":("pypdf",),
 "pdf.render":("pypdfium2","PIL"), "image.inspect":("PIL",), "image.compare":("PIL",)
}
ALIASES={
 "office.docx":"word report letter proposal document docx",
 "office.xlsx":"excel spreadsheet workbook xlsx tabular data",
 "office.pptx":"powerpoint slides presentation deck pptx",
 "pdf.generate":"create write produce generate report pdf",
 "pdf.render":"preview raster render pdf page image",
 "pdf.extract_text":"read extract pdf content text",
 "pdf.inspect":"pdf page dimensions count metadata",
 "pdf.merge":"combine join merge pdf documents",
 "pdf.split":"split select extract pdf pages",
 "image.inspect":"image preview brightness whitespace bounds",
 "image.compare":"image compare visual revision differences pixels",
 "artifact.inspect":"validate file integrity structure document office pdf",
 "artifact.hash":"checksum fingerprint hash integrity",
 "data.describe":"statistics average mean median numerical analysis",
 "math.calculate":"arithmetic calculate calculator sum multiply divide",
 "code.dependencies":"dependencies libraries packages npm composer cargo",
 "project.detect":"repository framework language project technology",
 "python.symbols":"python functions classes definitions symbols",
 "python.imports":"python import modules dependencies",
 "layout.bounds":"layout clipping margin overflow boundary",
 "layout.overlap":"layout collision intersection overlap",
 "design.contrast":"colour color accessibility contrast wcag",
 "csv.profile":"csv table missing data columns",
 "artifact.list":"list stored documents artifacts files",
 "artifact.metadata":"artifact file metadata version reference"
}
WORKFLOWS={"pdf_preview":"create generate pdf report document preview render verify validation"}

def obj(properties):
    return {"type":"object","properties":properties,"required":list(properties),"additionalProperties":False}

ID={"type":"string","pattern":"^[0-9a-f]{32}$","maxLength":32}

def catalog():
    tools={}
    for name,(folder,manifest,tool) in registry().items():
        spec=copy.deepcopy(tool)
        schema=spec["input_schema"]
        props=schema["properties"]
        if name in SINGLE:
            props.pop("content")
            props["artifact_id"]=copy.deepcopy(ID)
            schema["required"]=[k if k!="content" else "artifact_id" for k in schema["required"]]
        elif name=="pdf.merge":
            spec["input_schema"]=obj({"artifact_ids":{"type":"array","minItems":1,"maxItems":5,"items":copy.deepcopy(ID)}})
        elif name=="image.compare":
            spec["input_schema"]=obj({"first_id":copy.deepcopy(ID),"second_id":copy.deepcopy(ID)})
        tools[name]={"definition":spec,"category":folder.name,"dependencies":DEPENDENCIES.get(name,())}
    for name,props in (("artifact.list",{}),("artifact.metadata",{"artifact_id":copy.deepcopy(ID)})):
        tools[name]={"definition":{"name":name,"description":ALIASES[name],"input_schema":obj(props)},"category":"artifacts","dependencies":()}
    for name,schema in SCHEMAS.items():
        tools[name]={"definition":{"name":name,"description":name.replace("."," ")+" read-only diagnostics or model memory estimate","input_schema":copy.deepcopy(schema)},"category":"system","dependencies":()}
    return tools

def tokens(value):
    return set(re.findall(r"[a-z0-9]+",value.casefold()))

def missing(deps):
    result=[]
    for module in deps:
        try:
            if importlib.util.find_spec(module) is None: result.append(module)
        except (ImportError,ValueError): result.append(module)
    return result

def discover(query,allowed,limit=6,budget_bytes=6000,available_only=True):
    """Return at most limit definitions within a UTF-8 JSON byte budget.
    allowed must come from the trusted host, not the model.
    Dependency checks test discoverability only, not successful import/execution.
    """
    if type(query) is not str or len(query)>4000 or type(limit) is not int or not 1<=limit<=20 or type(budget_bytes) is not int or not 2<=budget_bytes<=32000: raise ValueError("Invalid discovery limits")
    if not isinstance(allowed,(set,frozenset)): raise ValueError("Explicit host allowlist required")
    terms=tokens(query)
    candidates=[]
    for name,entry in catalog().items():
        if name not in allowed: continue
        absent=missing(entry["dependencies"])
        if available_only and absent: continue
        score=4*len(terms & tokens(name))+2*len(terms & tokens(ALIASES.get(name,"")))+len(terms & tokens(entry["definition"]["description"]))
        if name.casefold() in query.casefold(): score+=20
        if score: candidates.append((-score,name,entry,absent))
    selected=[]
    for _,name,entry,absent in sorted(candidates,key=lambda x:(x[0],x[1])):
        item={"definition":entry["definition"],"category":entry["category"],"missing_modules":absent}
        trial=selected+[item]
        if len(json.dumps(trial,ensure_ascii=False,separators=(",",":")).encode())<=budget_bytes:
            selected=trial
        if len(selected)>=limit: break
    return selected

def match_workflows(query,allowed_workflows):
    if type(query) is not str or len(query)>4000 or not isinstance(allowed_workflows,(set,frozenset)): raise ValueError("Invalid workflow discovery")
    terms=tokens(query)
    # Require explicit PDF intent plus production or preview intent.
    return [name for name in WORKFLOWS if name in allowed_workflows and "pdf" in terms and terms & {"create","generate","preview","render","produce"}]
