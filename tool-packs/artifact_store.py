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
from pathlib import Path

FORMATS={"pdf","docx","xlsx","pptx","png","jpg"}
MAX_BYTES=700000
MAX_TOTAL=20*1024*1024
MAX_COUNT=100

class ArtifactStore:
    def __init__(self, private_parent):
        parent=Path(private_parent).resolve(strict=True)
        if not parent.is_dir(): raise ValueError("Invalid private parent")
        self._root=Path(tempfile.mkdtemp(prefix="vmt-artifacts-",dir=parent))
        self._records={}
        self._lock=threading.RLock()

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
            except BaseException:
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
