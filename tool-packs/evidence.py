"""Bounded source snapshots and exact passage references.
Optional private collection persistence; plaintext, not claim verification.
Only trusted retrieval code registers fetched evidence.
"""
import json
import os
import tempfile
from pathlib import Path
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
    def __init__(self,private_directory=None):
        self._records={}
        self._lock=threading.RLock()
        self._path=None
        if private_directory is not None:
            root=Path(private_directory)
            if not root.is_dir(): raise ValueError("Invalid research directory")
            self._path=root/"research-evidence.json"
            self._check_path()
            if self._path.exists():
                with self._path.open("rb") as stream:
                    raw=stream.read(24000001)
                if len(raw)>24000000: raise ValueError("Research store exceeds limit")
                rows=json.loads(raw)
                if type(rows) is not list or len(rows)>MAX_SOURCES: raise ValueError("Invalid research store")
                for row in rows:
                    self._validate_record(row)
                    if row["source_id"] in self._records: raise ValueError("Duplicate source ID")
                    self._records[row["source_id"]]=row

    def _check_path(self):
        path=self._path
        if path.is_symlink() or (path.exists() and (not path.is_file() or getattr(path.lstat(),"st_file_attributes",0)&0x400 or path.stat().st_nlink!=1)):
            raise ValueError("Invalid research store path")

    @staticmethod
    def _validate_record(row):
        from research_adapter import parse_url
        fields={"source_id","url","title","text","response_sha256","text_sha256","retrieved_at","truncated","trust"}
        if type(row) is not dict or set(row)!=fields: raise ValueError("Invalid source record")
        for key,bound in (("url",4096),("title",300),("text",MAX_TEXT),("retrieved_at",64)):
            if type(row[key]) is not str or len(row[key])>bound: raise ValueError("Invalid source text")
        parse_url(row["url"])
        if not re.fullmatch("[0-9a-f]{32}",row["source_id"]) or not re.fullmatch("[0-9a-f]{64}",row["response_sha256"]): raise ValueError("Invalid source identity")
        if row["text_sha256"]!=hashlib.sha256(row["text"].encode()).hexdigest(): raise ValueError("Research text digest mismatch")
        if type(row["truncated"]) is not bool or row["trust"]!="untrusted_external_content": raise ValueError("Invalid source status")

    def _persist(self,records):
        if self._path is None: return
        self._check_path()
        data=json.dumps(list(records.values()),ensure_ascii=False,allow_nan=False).encode("utf-8")
        if len(data)>24000000: raise ValueError("Research store exceeds limit")
        tmp=None
        try:
            with tempfile.NamedTemporaryFile(dir=self._path.parent,prefix=".research-",delete=False) as stream:
                tmp=Path(stream.name)
                stream.write(data);stream.flush();os.fsync(stream.fileno())
            self._check_path()
            os.replace(tmp,self._path)
        finally:
            if tmp is not None: tmp.unlink(missing_ok=True)

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
            self._validate_record(r)
            updated={**self._records,sid:r}
            self._persist(updated)
            self._records=updated
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
