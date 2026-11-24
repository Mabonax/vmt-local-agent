"""Session-local source snapshots and exact passage references.
Only trusted retrieval code registers fetched evidence. No model-supplied
provenance, no claim verification, no persistent storage.
"""
import copy
import hashlib
import re
import threading
import uuid
from datetime import datetime, timezone

MAX_SOURCES=100
MAX_TEXT=30000

def obj(props):
    return {"type":"object","properties":props,"required":list(props),"additionalProperties":False}
ID={"type":"string","maxLength":32,"pattern":"^[0-9a-f]{32}$"}
SCHEMAS={
 "research.sources":obj({}),
 "research.find":obj({"query":{"type":"string","maxLength":300},"limit":{"type":"integer","minimum":1,"maximum":20}}),
 "research.citation":obj({"source_id":ID,"start":{"type":"integer","minimum":0,"maximum":MAX_TEXT},"end":{"type":"integer","minimum":1,"maximum":MAX_TEXT}})
}

class EvidenceStore:
    def __init__(self):
        self._records={}
        self._lock=threading.RLock()

    def register_fetch(self,result):
        """Trusted host only; called directly after the fetch adapter succeeds."""
        from research_adapter import parse_url
        url=result["url"]; parse_url(url)
        title=result["title"]; text=result["text"]; digest=result["sha256"]
        if type(title) is not str or len(title)>300 or type(text) is not str or len(text)>MAX_TEXT or type(digest) is not str or not re.fullmatch("[0-9a-f]{64}",digest): raise ValueError("Invalid source snapshot")
        if type(result["truncated"]) is not bool: raise ValueError("Invalid source status")
        with self._lock:
            for record in self._records.values():
                if record["url"]==url and record["response_sha256"]==digest and record["text"]==text:
                    return self._metadata(record)
            if len(self._records)>=MAX_SOURCES: raise ValueError("Source quota exceeded")
            sid=uuid.uuid4().hex
            r={"source_id":sid,"url":url,"title":title,"text":text,
               "response_sha256":digest,"text_sha256":hashlib.sha256(text.encode()).hexdigest(),
               "retrieved_at":datetime.now(timezone.utc).isoformat(),
               "truncated":result["truncated"],"trust":"untrusted_external_content"}
            self._records[sid]=r
            return self._metadata(r)

    def _metadata(self,r):
        return {k:v for k,v in r.items() if k!="text"}

    def sources(self):
        with self._lock: return [self._metadata(r) for r in self._records.values()]

    def citation(self,source_id,start,end):
        with self._lock:
            r=self._records.get(source_id)
            if r is None or type(start) is not int or type(end) is not int or not 0<=start<end<=len(r["text"]) or end-start>1200: raise ValueError("Invalid passage reference")
            return {**self._metadata(r),"start":start,"end":end,"excerpt":r["text"][start:end]}

    def find(self,query,limit):
        if type(query) is not str or not 1<=len(query)<=300 or type(limit) is not int or not 1<=limit<=20: raise ValueError("Invalid search")
        terms=set(re.findall(r"\w+",query.casefold()))
        if not terms: return []
        hits=[]
        with self._lock:
            for sid,r in self._records.items():
                text=r["text"]
                # Fixed overlapping windows preserve original Unicode offsets.
                for start in range(0,len(text),600):
                    end=min(start+800,len(text))
                    words=set(re.findall(r"\w+",text[start:end].casefold()))
                    score=len(terms & words)
                    if score: hits.append((-score,sid,start,end))
            hits.sort()
            return [self.citation(sid,start,end) for _,sid,start,end in hits[:limit]]
