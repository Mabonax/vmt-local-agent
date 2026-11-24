"""Operator-only synthetic review drift and cancellation acceptance.

Run after verify_unified_integration.py, with its isolated profile closed in the
desktop. This changes only synthetic drift.txt and generated request/report files.
"""
import argparse,json,pathlib,subprocess,time
import psutil

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--checker',required=True);parser.add_argument('--fixture',required=True)
    args=parser.parse_args();checker=str(pathlib.Path(args.checker).resolve());fixture=pathlib.Path(args.fixture).resolve()
    if '.local' not in fixture.parts:raise ValueError('Use an isolated .local fixture')
    workspace=fixture/'project';base=['--data-dir',str(fixture/'profile')]
    source=workspace/'drift.txt';source.write_text('Approved source')
    request=fixture/'drift-request.json';request.write_text(json.dumps({'tool':'knowledge.ingest','arguments':{'path':'drift.txt','title':'Drift acceptance'}}))
    process=subprocess.Popen([checker,'--tool-pack-run',str(workspace),str(request),'--review-stdin',*base],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
    # The exact reviewed plan is emitted before the input gate. A watchdog bounds
    # transport failure without ever approving a different action.
    import threading
    watchdog=threading.Timer(25,process.kill);watchdog.start()
    try:
        while True:
            line=process.stdout.readline()
            if 'Prepared exact action.' in line:break
            if not line:raise RuntimeError('Review did not become ready')
        source.write_text('Concurrent replacement')
        _,stderr=process.communicate('APPROVE\n',timeout=20)
    finally:watchdog.cancel()
    assert process.returncode!=0 and 'Input changed after review' in stderr
    assert source.read_text()=='Concurrent replacement'
    print('Reviewed input drift rejected',flush=True)
    doc=json.loads((fixture/'result-01.json').read_text())['result']
    request=fixture/'cancel-request.json';request.write_text(json.dumps({'tool':'office.preview','arguments':{'artifact_id':doc['artifact_id']}}))
    started=time.monotonic();process=subprocess.Popen([checker,'--tool-pack-run',str(workspace),str(request),'--cancel-after-ms','750',*base],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
    seen={}
    while process.poll() is None:
        if time.monotonic()-started>125:process.kill();raise TimeoutError('Cancellation deadline')
        try:
            for child in psutil.Process(process.pid).children(recursive=True):seen[child.pid]=(child.name(),child.create_time())
        except psutil.Error:pass
        time.sleep(.02)
    _,stderr=process.communicate(timeout=5);time.sleep(.3)
    alive=[]
    for pid,(_,created) in seen.items():
        try:
            if psutil.Process(pid).create_time()==created:alive.append(pid)
        except psutil.Error:pass
    assert process.returncode!=0 and 'cancel' in stderr.lower() and not alive,(process.returncode,stderr,alive)
    assert any(name in ('soffice.exe','soffice.bin','soffice') for name,_ in seen.values()),'No active Office descendant observed; repeat this scoped trial'
    result={'input_drift':'PASS','cancellation':'PASS','seconds':time.monotonic()-started,'owned_descendants':{pid:name for pid,(name,_) in seen.items()},'surviving_descendants':alive}
    (fixture/'security-report.json').write_text(json.dumps(result,indent=2));print(json.dumps(result))

if __name__=='__main__':main()
