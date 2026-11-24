"""Fixed trusted leaf entry point: apply POSIX limits before importing a runner."""
import os
import pathlib
import runpy
import sys

if os.name!='nt':
    import resource
    resource.setrlimit(resource.RLIMIT_AS,(512*1024*1024,512*1024*1024))
    resource.setrlimit(resource.RLIMIT_CPU,(12,12))
    resource.setrlimit(resource.RLIMIT_FSIZE,(16*1024*1024,16*1024*1024))
root=pathlib.Path(__file__).resolve().parent
script=pathlib.Path(sys.argv[1]).resolve(strict=True)
if script.parent.parent!=root or script.name!='runner.py': raise ValueError('Untrusted leaf')
sys.argv=[str(script)]
runpy.run_path(str(script),run_name='__main__')
