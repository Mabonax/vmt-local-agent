"""Host-only artifact-aware tool execution. No model-controlled paths."""
from dispatch import dispatch

PRODUCERS={"office.docx":"docx","office.xlsx":"xlsx","office.pptx":"pptx",
           "pdf.generate":"pdf","pdf.merge":"pdf","pdf.split":"pdf","pdf.render":"png"}
SINGLE={"pdf.inspect","pdf.extract_text","pdf.split","artifact.inspect","artifact.hash","pdf.render","image.inspect"}

class ArtifactHost:
    def __init__(self, store, cancel=None):
        self.store=store
        self.cancel=cancel

    def call(self, name, args, previous=None):
        if name=="artifact.list":
            if args!={} or previous is not None: raise ValueError("Invalid arguments")
            return {"ok":True,"result":{"artifacts":self.store.list()}}
        if name=="artifact.metadata":
            if type(args) is not dict or set(args)!={"artifact_id"} or previous is not None: raise ValueError("Invalid arguments")
            return {"ok":True,"result":self.store.metadata(args["artifact_id"])}
        if type(args) is not dict: raise ValueError("Invalid arguments")
        args=dict(args)
        if previous is not None and name not in PRODUCERS: raise ValueError("Unexpected version reference")
        if name in SINGLE:
            if "content" in args or "artifact_id" not in args: raise ValueError("Use artifact reference")
            artifact_id=args.pop("artifact_id")
            if name.startswith("pdf.") and self.store.metadata(artifact_id)["extension"]!="pdf": raise ValueError("Expected PDF")
            if name=="image.inspect" and self.store.metadata(artifact_id)["extension"] not in ("png","jpg"): raise ValueError("Expected image")
            args["content"]=self.store.retrieve(artifact_id)
        elif name=="image.compare":
            if set(args)!={"first_id","second_id"}: raise ValueError("Use image references")
            ids=[args["first_id"],args["second_id"]]
            if any(self.store.metadata(i)["extension"] not in ("png","jpg") for i in ids): raise ValueError("Expected images")
            args={"first":self.store.retrieve(ids[0]),"second":self.store.retrieve(ids[1])}
        elif name=="pdf.merge":
            if set(args)!={"artifact_ids"}: raise ValueError("Use artifact references")
            ids=args["artifact_ids"]
            if type(ids) is not list or not 1<=len(ids)<=5: raise ValueError("Invalid references")
            if any(self.store.metadata(i)["extension"]!="pdf" for i in ids): raise ValueError("Expected PDF")
            args={"documents":[self.store.retrieve(i) for i in ids]}
        response=dispatch(name,args,cancel=self.cancel)
        if response["ok"] and name in PRODUCERS:
            if response["result"].get("extension")!=PRODUCERS[name]: raise ValueError("Unexpected artifact format")
            return {"ok":True,"result":self.store.store(response["result"],previous)}
        return response
