"""Fingerprint trusted pack scripts and manifests against a host-pinned baseline.
A baseline must be obtained from a reviewed release, never from unknown files.
Not code signing, dependency verification, or race-proof isolation.
"""
import hashlib
import stat
from dispatch import ROOT,TRUSTED

def snapshot():
    result={}
    for pack in TRUSTED:
        folder=ROOT/pack
        if folder.is_symlink() or getattr(folder.lstat(),"st_file_attributes",0)&0x400: raise ValueError("Linked pack")
        for name in ("manifest.json","runner.py"):
            path=folder/name
            info=path.lstat()
            if not stat.S_ISREG(info.st_mode) or getattr(info,"st_file_attributes",0)&0x400 or info.st_size>1048576: raise ValueError("Invalid pack file")
            with path.open("rb") as f: data=f.read(1048577)
            if len(data)>1048576: raise ValueError("Pack file too large")
            result[pack+"/"+name]=hashlib.sha256(data).hexdigest()
    return result

def verify(expected):
    if type(expected) is not dict or snapshot()!=expected: raise PermissionError("Tool integrity mismatch")
