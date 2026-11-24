"""Owned Windows Job Object. Limits do not deny filesystem or network access."""
import os

class WindowsJob:
    def __init__(self, process, memory_bytes=512*1024*1024, cpu_seconds=12):
        self.handle=None
        if os.name!='nt': return
        import ctypes as c
        from ctypes import wintypes as w
        class Basic(c.Structure):
            _fields_=[('ProcessTime',c.c_int64),('JobTime',c.c_int64),('Flags',w.DWORD),('MinWorking',c.c_size_t),('MaxWorking',c.c_size_t),('Active',w.DWORD),('Affinity',c.c_size_t),('Priority',w.DWORD),('Scheduling',w.DWORD)]
        class IO(c.Structure):
            _fields_=[(name,c.c_uint64) for name in ('ReadOps','WriteOps','OtherOps','ReadBytes','WriteBytes','OtherBytes')]
        class Extended(c.Structure):
            _fields_=[('Basic',Basic),('IO',IO),('ProcessMemory',c.c_size_t),('JobMemory',c.c_size_t),('PeakProcess',c.c_size_t),('PeakJob',c.c_size_t)]
        self.api=c.WinDLL('kernel32',use_last_error=True)
        self.api.CreateJobObjectW.argtypes=[c.c_void_p,w.LPCWSTR]; self.api.CreateJobObjectW.restype=w.HANDLE
        self.api.SetInformationJobObject.argtypes=[w.HANDLE,c.c_int,c.c_void_p,w.DWORD]; self.api.SetInformationJobObject.restype=w.BOOL
        self.api.AssignProcessToJobObject.argtypes=[w.HANDLE,w.HANDLE]; self.api.AssignProcessToJobObject.restype=w.BOOL
        self.api.CloseHandle.argtypes=[w.HANDLE]; self.api.CloseHandle.restype=w.BOOL
        self.handle=self.api.CreateJobObjectW(None,None)
        if not self.handle: raise OSError('Cannot create process job')
        limits=Extended(); limits.Basic.Flags=0x2000|0x100 # KILL_ON_JOB_CLOSE | PROCESS_MEMORY
        limits.ProcessMemory=memory_bytes
        if cpu_seconds:
            limits.Basic.Flags|=2; limits.Basic.ProcessTime=int(cpu_seconds*10000000)
        try:
            if not self.api.SetInformationJobObject(self.handle,9,c.byref(limits),c.sizeof(limits)) or not self.api.AssignProcessToJobObject(self.handle,w.HANDLE(int(process._handle))):
                raise OSError('Cannot contain owned process')
        except BaseException:
            self.close(); process.kill(); process.wait(); raise

    def close(self):
        if self.handle:
            self.api.CloseHandle(self.handle); self.handle=None

    def __del__(self): self.close()
