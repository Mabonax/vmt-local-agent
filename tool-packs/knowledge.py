"""Persistent local lexical retrieval. One host-owned database per collection.
Ingestion is a trusted host operation; model-facing tools are read-only.
Database directory must be private and not writable by untrusted actors.
"""
import hashlib
import re
import sqlite3
import threading
import uuid
import stat
from pathlib import Path
from datetime import datetime,timezone

def obj(p):
    return {"type":"object","properties":p,"required":list(p),"additionalProperties":False}
def integer(a,b):
    return {"type":"integer","minimum":a,"maximum":b}
ID={"type":"string","maxLength":32,"pattern":"^[0-9a-f]{32}$"}
SCHEMAS={
 "knowledge.search":obj({"query":{"type":"string","maxLength":300},"limit":integer(1,20)}),
 "knowledge.documents":obj({"offset":integer(0,500),"limit":integer(1,50)}),
 "knowledge.citation":obj({"document_id":ID,"start":integer(0,100000),"end":integer(1,100000)}),
 "knowledge.statistics":obj({})
}

class KnowledgeStore:
    def __init__(self,private_directory):
        root=Path(private_directory).resolve(strict=True)
        if not root.is_dir(): raise ValueError("Invalid private directory")
        path=root/"knowledge.sqlite3"
        if path.is_symlink() or (path.exists() and (not path.is_file() or getattr(path.lstat(),'st_file_attributes',0)&0x400 or path.stat().st_nlink!=1)): raise ValueError("Invalid database")
        self._lock=threading.RLock()
        self._db=sqlite3.connect(path,timeout=5,check_same_thread=False)
        self._db.row_factory=sqlite3.Row
        self._db.executescript("""
          PRAGMA foreign_keys=ON;
          CREATE TABLE IF NOT EXISTS documents(
            id TEXT PRIMARY KEY, source_key TEXT NOT NULL, title TEXT NOT NULL,
            text TEXT NOT NULL, sha256 TEXT NOT NULL, revision INTEGER NOT NULL,
            indexed_at TEXT NOT NULL, UNIQUE(source_key,revision));
          CREATE VIRTUAL TABLE IF NOT EXISTS passages USING fts5(
            document_id UNINDEXED, start UNINDEXED, end UNINDEXED, text);
        """)
        if 'source_sha256' not in {row[1] for row in self._db.execute('PRAGMA table_info(documents)')}:
            self._db.execute('ALTER TABLE documents ADD COLUMN source_sha256 TEXT')
    def close(self):
        with self._lock: self._db.close()
    def ingest(self,source_key,title,text,source_sha256=None):
        """Host-only ingestion of explicitly authorized text, not file paths."""
        for value,limit in ((source_key,200),(title,300),(text,100000)):
            if type(value) is not str or not value.strip() or len(value)>limit or "\x00" in value: raise ValueError("Invalid document")
        digest=hashlib.sha256(text.encode()).hexdigest()
        if source_sha256 is not None and (type(source_sha256) is not str or not re.fullmatch('[0-9a-f]{64}',source_sha256)): raise ValueError('Invalid source digest')
        with self._lock,self._db:
            self._db.execute("BEGIN IMMEDIATE")
            last=self._db.execute("SELECT * FROM documents WHERE source_key=? ORDER BY revision DESC LIMIT 1",(source_key,)).fetchone()
            if last and last["sha256"]==digest and last["title"]==title and last['source_sha256']==source_sha256: return self._meta(last)
            count,size=self._db.execute("SELECT count(*),coalesce(sum(length(text)),0) FROM documents").fetchone()
            if count>=500 or size+len(text)>10000000: raise ValueError("Knowledge quota exceeded")
            doc=uuid.uuid4().hex
            rev=last["revision"]+1 if last else 1
            self._db.execute("INSERT INTO documents(id,source_key,title,text,sha256,revision,indexed_at,source_sha256) VALUES(?,?,?,?,?,?,?,?)",(doc,source_key,title,text,digest,rev,datetime.now(timezone.utc).isoformat(),source_sha256))
            self._db.executemany("INSERT INTO passages VALUES(?,?,?,?)",((doc,i,min(i+800,len(text)),text[i:i+800]) for i in range(0,len(text),600)))
            return self._meta(self._db.execute("SELECT * FROM documents WHERE id=?",(doc,)).fetchone())
    @staticmethod
    def _meta(row):
        return {"document_id":row["id"],"title":row["title"],"revision":row["revision"],"sha256":row["sha256"],"source_sha256":row['source_sha256'],"indexed_at":row["indexed_at"],"trust":"untrusted_document_content"}
    def documents(self,offset,limit):
        with self._lock:
            return [self._meta(r) for r in self._db.execute("SELECT * FROM documents ORDER BY indexed_at DESC,id LIMIT ? OFFSET ?",(limit,offset))]
    def citation(self,doc,start,end):
        with self._lock:
            r=self._db.execute("SELECT * FROM documents WHERE id=?",(doc,)).fetchone()
            if r is None or type(start) is not int or type(end) is not int or not 0<=start<end<=len(r["text"]) or end-start>1200: raise ValueError("Invalid citation")
            return {**self._meta(r),"start":start,"end":end,"excerpt":r["text"][start:end]}
    def search(self,query,limit):
        if type(query) is not str or len(query)>300 or type(limit) is not int or not 1<=limit<=20: raise ValueError("Invalid query")
        terms=list(dict.fromkeys(re.findall(r"[^\W_]+",query)))[:20]
        if not terms: return []
        expression=" OR ".join('"'+t+'"' for t in terms)
        with self._lock:
            rows=self._db.execute("""SELECT p.document_id,p.start,p.end FROM passages p
              JOIN documents d ON d.id=p.document_id
              WHERE passages MATCH ? AND NOT EXISTS
              (SELECT 1 FROM documents newer WHERE newer.source_key=d.source_key AND newer.revision>d.revision)
              ORDER BY bm25(passages),p.document_id,p.start LIMIT ?""",(expression,limit)).fetchall()
            return [self.citation(r[0],int(r[1]),int(r[2])) for r in rows]
    def statistics(self):
        with self._lock:
            n,c=self._db.execute("SELECT count(*),count(DISTINCT source_key) FROM documents").fetchone()
            p=self._db.execute("SELECT count(*) FROM passages").fetchone()[0]
            return {"revisions":n,"documents":c,"indexed_passages":p,"retrieval":"sqlite_fts5_bm25"}

class KnowledgeHost:
    def __init__(self,delegate,store):
        self._delegate=delegate; self._store=store
    def call(self,name,args,previous=None):
        if name not in SCHEMAS: return self._delegate.call(name,args,previous=previous)
        from dispatch import validate
        if previous is not None: raise ValueError("Unexpected revision argument")
        validate(args,SCHEMAS[name])
        if name=="knowledge.search": result={"passages":self._store.search(args["query"],args["limit"])}
        elif name=="knowledge.documents": result={"documents":self._store.documents(args["offset"],args["limit"])}
        elif name=="knowledge.citation": result=self._store.citation(args["document_id"],args["start"],args["end"])
        else: result=self._store.statistics()
        return {"ok":True,"result":result}
