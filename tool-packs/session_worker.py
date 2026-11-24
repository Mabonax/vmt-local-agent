"""Private JSONL service. Only the trusted Rust host owns this pipe protocol.

Never expose approve/export/init or raw artifact envelopes as model tools.
"""
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
import base64
import json
import os
import signal
import threading

MAX_WIRE=2000000
cancel=threading.Event()

def private_directory(path):
    # Do not resolve away a link before inspecting its components.
    for candidate in (path,*path.parents):
        if candidate.exists() or candidate.is_symlink():
            info=candidate.lstat()
            if candidate.is_symlink() or getattr(info,'st_file_attributes',0)&0x400:
                raise ValueError('Linked private directory rejected')
    path.mkdir(parents=True,exist_ok=True)
    if not path.is_dir(): raise ValueError('Invalid private directory')
if os.name!='nt':
    import resource
    resource.setrlimit(resource.RLIMIT_AS,(2*1024**3,2*1024**3))
    signal.signal(signal.SIGTERM,lambda *_:cancel.set())

class Session:
    def __init__(self,args):
        from discovery import catalog
        self.root=Path(args['state_directory']); self.workspace=Path(args['workspace'])
        self.collection=args['collection']; self.soffice=args.get('soffice')
        self.allowed=frozenset(args['allowed'])
        if not self.allowed<=set(catalog()): raise ValueError('Invalid host allowlist')
        self._policy=None; self._store=None; self._knowledge=None
        self._lease=None
        self._snapshots={}; self._research=None; self._adapter=None

    def host(self):
        if self._policy is not None: return self._policy
        from artifact_store import ArtifactStore
        from artifact_host import ArtifactHost
        from system_adapter import SystemHost
        from research_adapter import ResearchHost
        from knowledge import KnowledgeStore,KnowledgeHost
        from host_adapter import HostAdapter
        from policy import PolicyHost
        from integrity import snapshot,verify
        import re
        if not re.fullmatch('[a-zA-Z0-9_-]{1,40}',self.collection): raise ValueError('Invalid collection')
        private_directory(self.root)
        if self._lease is None:
            lock=self.root/'session.lock'
            if lock.is_symlink() or (lock.exists() and (getattr(lock.lstat(),'st_file_attributes',0)&0x400 or lock.stat().st_nlink!=1)):
                raise ValueError('Invalid session lock')
            lease=lock.open('a+b')
            if lock.stat().st_size==0: lease.write(b'0');lease.flush()
            lease.seek(0)
            try:
                if os.name=='nt':
                    import msvcrt
                    msvcrt.locking(lease.fileno(),msvcrt.LK_NBLCK,1)
                else:
                    import fcntl
                    fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
            except BaseException:
                lease.close();raise PermissionError('This workspace session is active in another process')
            self._lease=lease
        collection=self.root/'collections'/self.collection; private_directory(collection)
        self._store=ArtifactStore(self.root,persistent=True)
        self._knowledge=KnowledgeStore(collection)
        host=ArtifactHost(self._store,cancel)
        host=SystemHost(host,self.workspace)
        self._research=ResearchHost(host,set())
        host=KnowledgeHost(self._research,self._knowledge)
        self._adapter=HostAdapter(host,self._store,self._knowledge,self.workspace,self.soffice,cancel)
        self._adapter.research=self._research
        baseline=snapshot()
        self._policy=PolicyHost(self._adapter,self.allowed,review_required=self.allowed,integrity_check=lambda:verify(baseline))
        return self._policy

    def close(self):
        if self._knowledge is not None: self._knowledge.close();self._knowledge=None
        if self._lease is not None: self._lease.close();self._lease=None

    def request(self,request):
        action=request['action']
        if action=='discover':
            from discovery import discover,match_workflows
            allowed=self.allowed if request.get('internet_enabled') else self.allowed-{'web.fetch'}
            selected=discover(request['query'],set(allowed),limit=request.get('limit',6),budget_bytes=request.get('budget_bytes',6000))
            if not self.soffice: selected=[x for x in selected if x['definition']['name']!='office.preview']
            return {'tools':selected,'workflows':match_workflows(request['query'],{'pdf_preview'})}
        if action=='status':
            from discovery import catalog,missing
            tools=catalog()
            return {'tools':[{'name':name,'category':v['category'],'missing_modules':missing(v['dependencies'])} for name,v in tools.items() if name in self.allowed], 'workspace_operations':3,'office_preview':bool(self.soffice),'collection':self.collection,'process_isolation':'bounded process tree and resource controls; no filesystem/network sandbox'}
        if action=='approve':
            policy=self.host()
            token=policy.approve(request['tool'],request['arguments'],request.get('previous'))
            attachment=request.get('attachment')
            snapshot=base64.b64decode(attachment,validate=True) if attachment is not None else None
            if snapshot is not None and len(snapshot)>1048576: raise ValueError('Attachment exceeds limit')
            self._snapshots[token]=snapshot
            return {'approval':token}
        if action=='call':
            policy=self.host(); token=request.get('approval')
            self._adapter.input_bytes=self._snapshots.pop(token,None)
            try:
                result=policy.call(request['tool'],request['arguments'],previous=request.get('previous'),approval=token)
            finally:
                self._adapter.input_bytes=None
                self._research._hosts=frozenset()
            # No binary envelopes or private approval fields may reach reasoning.
            if len(json.dumps(result,ensure_ascii=False).encode())>32000: raise ValueError('Model result exceeds limit')
            return result
        if action=='artifacts':
            self.host(); return {'artifacts':self._store.list()}
        if action=='export':
            self.host(); return {'metadata':self._store.metadata(request['artifact_id']),'content':self._store.retrieve(request['artifact_id'])}
        if action=='receive_artifact':
            self.host()
            envelope=request['artifact']
            raw=base64.b64decode(envelope['content'],validate=True)
            if len(raw)>700000: raise ValueError('Artifact exceeds limit')
            self._adapter.input_bytes=raw
            try: return self._adapter.call('artifact.import',{'path':'received.'+envelope['extension']})['result']
            finally: self._adapter.input_bytes=None
        raise ValueError('Unknown host action')

def main():
    session=None
    while not cancel.is_set():
        line=sys.stdin.buffer.readline(MAX_WIRE+1)
        if not line: break
        rid=None
        try:
            if len(line)>MAX_WIRE or not line.endswith(b'\n'): raise ValueError('Transport size')
            request=json.loads(line); rid=request['id']
            if request['action']=='init':
                if session is not None: raise ValueError('Already initialized')
                session=Session(request); result={'ready':True}
            else:
                if session is None: raise ValueError('Not initialized')
                result=session.request(request)
            output={'id':rid,'ok':True,'result':result}
        except Exception as error:
            # Exception text can contain document content, URLs or private paths.
            output={'id':rid,'ok':False,'error':{'code':type(error).__name__,'message':'Host request rejected or execution failed; inspect the reviewed schema, dependency diagnostics and input limits.'}}
        encoded=json.dumps(output,allow_nan=False,ensure_ascii=False).encode()
        if len(encoded)>MAX_WIRE: break
        sys.stdout.buffer.write(encoded+b'\n'); sys.stdout.buffer.flush()
    if session is not None: session.close()

if __name__=='__main__': main()
