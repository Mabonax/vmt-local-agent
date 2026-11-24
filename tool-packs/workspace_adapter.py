"""Read-only workspace adapter. Caller supplies trusted root, never model arguments.

Requires a stable, user-controlled workspace: path checks are not race-proof
against concurrent hostile filesystem changes. Do not use as an OS sandbox.
"""
import hashlib
import os
import pathlib
import stat

MAX_BYTES=1024*1024
DENIED={".git",".ssh",".aws",".local","node_modules","target",".venv","__pycache__"}

def allowed(name):
    lower=name.casefold()
    return lower not in DENIED and not lower.startswith((".env",".")) and lower not in {"id_rsa","id_ed25519","credentials","secrets.json"} and not lower.endswith((".pem",".key",".pfx",".p12"))

def resolve(root,relative):
    root=pathlib.Path(root).resolve(strict=True)
    if not root.is_dir(): raise ValueError("Invalid workspace")
    if not isinstance(relative,str) or len(relative)>1024 or "\\" in relative or ":" in relative: raise ValueError("Invalid path")
    if relative==".": return root
    parts=relative.split("/")
    if any(p in ("",".","..") or not allowed(p) for p in parts): raise ValueError("Path rejected")
    path=root
    for part in parts:
        path=path/part
        info=path.lstat()
        if stat.S_ISLNK(info.st_mode) or getattr(info,"st_file_attributes",0)&0x400: raise ValueError("Links rejected")
    if not path.resolve(strict=True).is_relative_to(root): raise ValueError("Path outside workspace")
    return path

def inspect(root,relative,operation):
    path=resolve(root,relative)
    info=path.stat()
    if operation=="metadata":
        if not (stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode)): raise ValueError("Special file rejected")
        return {"kind":"directory" if path.is_dir() else "file","bytes":info.st_size}
    if operation=="sha256":
        if not stat.S_ISREG(info.st_mode) or info.st_size>MAX_BYTES or info.st_nlink!=1: raise ValueError("File rejected")
        with path.open("rb") as f: data=f.read(MAX_BYTES+1)
        if len(data)>MAX_BYTES: raise ValueError("File too large")
        return {"sha256":hashlib.sha256(data).hexdigest(),"bytes":len(data)}
    if operation=="list":
        if not path.is_dir(): raise ValueError("Expected directory")
        result=[]; scanned=0; truncated=False
        with os.scandir(path) as entries:
            for entry in entries:
                scanned+=1
                if scanned>1000 or len(result)>=200: truncated=True; break
                if not allowed(entry.name): continue
                try:
                    info=entry.stat(follow_symlinks=False)
                    if stat.S_ISLNK(info.st_mode) or getattr(info,"st_file_attributes",0)&0x400: continue
                    if stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode):
                        result.append({"name":entry.name,"kind":"directory" if stat.S_ISDIR(info.st_mode) else "file"})
                except OSError: continue
        return {"entries":sorted(result,key=lambda e:e["name"]),"truncated":truncated}
    raise ValueError("Unknown operation")
