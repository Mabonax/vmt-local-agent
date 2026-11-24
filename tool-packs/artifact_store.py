"""Session-scoped artifact storage for a trusted host.

The host chooses a private directory outside the inspected workspace. This is
not an OS sandbox or an authorization service. Never expose roots or raw bytes
to the model. Each session owns a new directory; no shared writable roots.
"""
import base64
import hashlib
import re
import stat
import tempfile
import threading
import uuid
import json
import os
import time
from pathlib import Path

FORMATS={"pdf","docx","xlsx","pptx","png","jpg"}
MAX_BYTES=700000
MAX_TOTAL=20*1024*1024
MAX_COUNT=100

class ArtifactStore:
    def __init__(self, private_parent, persistent=False):
        parent=Path(private_parent).resolve(strict=True)
        if not parent.is_dir(): raise ValueError("Invalid private parent")
        self._persistent=persistent
        self._root=parent/"artifacts" if persistent else Path(tempfile.mkdtemp(prefix="vmt-artifacts-",dir=parent))
        if persistent: self._root.mkdir(exist_ok=True)
        if self._root.is_symlink() or getattr(self._root.lstat(),"st_file_attributes",0)&0x400:
            raise ValueError("Linked artifact storage")
        self._records={}
        self._lock=threading.RLock()
        index=self._root/"index.json"
        if persistent and index.exists():
            if index.is_symlink() or getattr(index.lstat(),"st_file_attributes",0)&0x400 or index.stat().st_size>100000:
                raise ValueError("Invalid artifact index")
            records=json.loads(index.read_text(encoding="utf-8"))
            if type(records) is not list or len(records)>MAX_COUNT: raise ValueError("Invalid artifact index")
            for record in records:
                if type(record) is not dict or set(record)!={"artifact_id","extension","bytes","sha256","previous","version"}: raise ValueError("Invalid artifact record")
                if type(record['artifact_id']) is not str or not re.fullmatch('[0-9a-f]{32}',record['artifact_id']) or record['artifact_id'] in self._records: raise ValueError("Invalid artifact ID")
                if record['extension'] not in FORMATS or type(record['bytes']) is not int or not 0<record['bytes']<=MAX_BYTES or not re.fullmatch('[0-9a-f]{64}',record['sha256']): raise ValueError("Invalid artifact metadata")
                parent_record=self._records.get(record['previous']) if record['previous'] is not None else None
                if record['previous'] is not None and (parent_record is None or parent_record['extension']!=record['extension']): raise ValueError("Invalid revision ancestry")
                if type(record['version']) is not int or record['version']!=(parent_record['version']+1 if parent_record else 1): raise ValueError("Invalid revision number")
                self._records[record['artifact_id']]=record
            if sum(r['bytes'] for r in self._records.values())>MAX_TOTAL: raise ValueError("Artifact quota exceeded")

    def _persist(self):
        if not self._persistent: return
        index=self._root/"index.json"
        if index.exists() and (index.is_symlink() or getattr(index.lstat(),"st_file_attributes",0)&0x400): raise ValueError("Linked index")
        fd,name=tempfile.mkstemp(prefix=".index-",dir=self._root)
        try:
            with os.fdopen(fd,'w',encoding='utf-8') as stream:
                json.dump(list(self._records.values()),stream); stream.flush(); os.fsync(stream.fileno())
            for attempt in range(5):
                try: os.replace(name,index); break
                except PermissionError:
                    if attempt==4: raise
                    time.sleep(.03*(attempt+1))
        finally:
            Path(name).unlink(missing_ok=True)

    def _record(self, artifact_id):
        if type(artifact_id) is not str or not re.fullmatch(r"[0-9a-f]{32}",artifact_id):
            raise ValueError("Invalid artifact reference")
        if artifact_id not in self._records: raise ValueError("Unknown artifact")
        return self._records[artifact_id]

    def store(self, result, previous=None):
        if type(result) is not dict or set(result)!={"extension","encoding","bytes","content"}:
            raise ValueError("Invalid artifact envelope")
        extension=result["extension"]
        if type(extension) is not str or extension not in FORMATS or result["encoding"]!="base64":
            raise ValueError("Unsupported artifact")
        encoded=result["content"]
        if type(encoded) is not str or len(encoded)>940000: raise ValueError("Invalid artifact content")
        data=base64.b64decode(encoded,validate=True)
        if not 0<len(data)<=MAX_BYTES or type(result["bytes"]) is not int or result["bytes"]!=len(data):
            raise ValueError("Artifact length mismatch")
        # Signature checks are not complete format or malware validation.
        signatures={"pdf":b"%PDF-","docx":b"PK\x03\x04","xlsx":b"PK\x03\x04","pptx":b"PK\x03\x04","png":b"\x89PNG\r\n\x1a\n","jpg":b"\xff\xd8"}
        if not data.startswith(signatures[extension]): raise ValueError("Invalid artifact signature")
        with self._lock:
            parent=self._record(previous) if previous is not None else None
            if parent and parent["extension"]!=extension: raise ValueError("Version format mismatch")
            if len(self._records)>=MAX_COUNT or sum(x["bytes"] for x in self._records.values())+len(data)>MAX_TOTAL:
                raise ValueError("Artifact quota exceeded")
            artifact_id=uuid.uuid4().hex
            record={"artifact_id":artifact_id,"extension":extension,"bytes":len(data),
                    "sha256":hashlib.sha256(data).hexdigest(),"previous":previous,
                    "version":parent["version"]+1 if parent else 1}
            path=self._root/(artifact_id+"."+extension)
            created=False
            try:
                with path.open("xb") as f:
                    created=True
                    f.write(data)
                self._records[artifact_id]=record
                self._persist()
            except BaseException:
                self._records.pop(artifact_id,None)
                if created: path.unlink(missing_ok=True)
                raise
            return dict(record)

    def metadata(self, artifact_id):
        with self._lock: return dict(self._record(artifact_id))

    def list(self):
        with self._lock: return [dict(r) for r in self._records.values()]

    def retrieve(self, artifact_id):
        """Host-only retrieval, with size and digest verification."""
        with self._lock:
            r=self._record(artifact_id)
            path=self._root/(artifact_id+"."+r["extension"])
            info=path.lstat()
            if not stat.S_ISREG(info.st_mode) or getattr(info,"st_file_attributes",0)&0x400 or info.st_nlink!=1 or info.st_size!=r["bytes"]:
                raise ValueError("Artifact storage changed")
            with path.open("rb") as f: data=f.read(MAX_BYTES+1)
            if len(data)!=r["bytes"] or hashlib.sha256(data).hexdigest()!=r["sha256"]:
                raise ValueError("Artifact integrity failure")
            return base64.b64encode(data).decode("ascii")
