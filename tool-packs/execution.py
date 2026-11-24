"""Bounded subprocess transport for trusted leaf runners only.
Not a sandbox: no hard CPU/RAM limit or Windows descendant containment.
"""
import os
import signal
import subprocess
import threading
import time

def run_bounded(argv,request,cwd,timeout,output_limit,cancel=None):
    if cancel is not None and cancel.is_set(): raise ValueError("Cancelled")
    if not 0<timeout<=10 or not 0<output_limit<=1048576 or len(request)>1048576: raise ValueError("Invalid execution limits")
    # Windows may require SystemRoot to load runtime libraries.
    env={k:os.environ[k] for k in ("SystemRoot","WINDIR") if os.name=="nt" and k in os.environ}
    options={"start_new_session":True} if os.name!="nt" else {}
    p=subprocess.Popen(argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=cwd,env=env,**options)
    overflow=threading.Event(); io_error=threading.Event()
    buffers=[bytearray(),bytearray()]
    def read(pipe,buf):
        try:
            while True:
                chunk=pipe.read1(8192)
                if not chunk: break
                remaining=output_limit-len(buf)
                buf.extend(chunk[:remaining])
                if len(chunk)>remaining:
                    overflow.set(); break
        except (OSError,ValueError): io_error.set()
        finally: pipe.close()
    def write():
        try:
            p.stdin.write(request); p.stdin.flush()
        except (BrokenPipeError,OSError,ValueError): pass
        finally: p.stdin.close()
    threads=[threading.Thread(target=read,args=(p.stdout,buffers[0]),daemon=True),threading.Thread(target=read,args=(p.stderr,buffers[1]),daemon=True),threading.Thread(target=write,daemon=True)]
    for t in threads: t.start()
    deadline=time.monotonic()+timeout
    def stop():
        if os.name!="nt":
            try: os.killpg(p.pid,signal.SIGKILL)
            except ProcessLookupError: pass
        elif p.poll() is None: p.kill()
    try:
        while p.poll() is None or any(t.is_alive() for t in threads):
            if overflow.is_set(): raise ValueError("Output limit exceeded")
            if io_error.is_set(): raise ValueError("Transport error")
            if cancel is not None and cancel.is_set(): raise ValueError("Cancelled")
            if time.monotonic()>=deadline: raise subprocess.TimeoutExpired(argv,timeout)
            time.sleep(.01)
        if overflow.is_set() or io_error.is_set(): raise ValueError("Output limit or transport failure")
        if cancel is not None and cancel.is_set(): raise ValueError("Cancelled")
        return subprocess.CompletedProcess(argv,p.returncode,bytes(buffers[0]),bytes(buffers[1]))
    finally:
        if p.poll() is None or any(t.is_alive() for t in threads): stop()
        p.wait(timeout=2)
        for t in threads: t.join(timeout=.1)
