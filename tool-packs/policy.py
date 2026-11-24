"""Host-owned execution authorization. Never expose approval issuance to models."""
import copy
import hashlib
import json
import secrets
import threading
import time
from collections import deque
from discovery import catalog
from dispatch import validate

def fingerprint(name,args,previous):
    payload=json.dumps([name,args,previous],sort_keys=True,allow_nan=False,separators=(",",":")).encode()
    if len(payload)>1048576: raise ValueError("Request too large")
    return hashlib.sha256(payload).hexdigest()

class PolicyHost:
    def __init__(self,host,allowed,review_required=(),integrity_check=None):
        self._host=host
        self._allowed=frozenset(allowed)
        self._review=frozenset(review_required)
        if not self._allowed<=set(catalog()) or not self._review<=self._allowed: raise ValueError("Invalid policy")
        self._integrity_check=integrity_check
        self._tokens={}
        self._journal=deque(maxlen=1000)
        self._lock=threading.RLock()

    def approve(self,name,args,previous=None):
        """Trusted UI only: call after explicit user review of the exact request."""
        with self._lock:
            if name not in self._review: raise PermissionError("Not a reviewable tool")
            now=time.monotonic()
            self._tokens={k:v for k,v in self._tokens.items() if v[1]>now}
            if len(self._tokens)>=100: raise ValueError("Too many pending approvals")
            token=secrets.token_urlsafe(32)
            self._tokens[token]=(fingerprint(name,args,previous),now+60)
            return token

    def journal(self):
        with self._lock: return list(copy.deepcopy(self._journal))

    def call(self,name,args,previous=None,approval=None):
        started=time.monotonic(); status="denied"
        label=name if isinstance(name,str) and name in self._allowed else "unknown_or_denied"
        try:
            if name not in self._allowed: raise PermissionError("Tool denied")
            args=copy.deepcopy(args)
            validate(args,catalog()[name]["definition"]["input_schema"])
            digest=fingerprint(name,args,previous)
            with self._lock:
                if name in self._review:
                    if type(approval) is not str: raise PermissionError("Approval required")
                    record=self._tokens.pop(approval,None)
                    if record is None or record[1]<=time.monotonic() or not secrets.compare_digest(record[0],digest): raise PermissionError("Invalid approval")
                elif approval is not None: raise PermissionError("Unexpected approval")
            if self._integrity_check is not None: self._integrity_check()
            status="error"
            result=self._host.call(name,args,previous=previous)
            status="passed" if result.get("ok") is True else "failed"
            return result
        finally:
            with self._lock:
                self._journal.append({"tool":label,"status":status,"duration_ms":round((time.monotonic()-started)*1000)})
