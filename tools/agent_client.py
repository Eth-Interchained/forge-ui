#!/usr/bin/env python3
"""Dependency-free example client. Starts the real native workbench and asserts an edit.
Usage: python3 tools/agent_client.py [path-to-showcase] [data-directory]
Requires a desktop display. NEDB state stays in the chosen directory.
"""
import json,pathlib,subprocess,sys
root=pathlib.Path(__file__).resolve().parents[1]
binary=sys.argv[1] if len(sys.argv)>1 else str(root/'target/release/examples'/('showcase.exe' if sys.platform=='win32' else 'showcase'))
data=sys.argv[2] if len(sys.argv)>2 else str(root/'.forge-agent-data')
p=subprocess.Popen([binary,'--agent','--data',data],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,bufsize=1)
def read(startup=False):
    while True:
        line=p.stdout.readline()
        if not line:raise RuntimeError('app exited before response')
        try:return json.loads(line)
        except json.JSONDecodeError:
            if not startup:raise
            print(line.rstrip(),file=sys.stderr)
def request(n,op,**fields):
    p.stdin.write(json.dumps(dict(request_id=str(n),op=op,**fields))+'\n');p.stdin.flush()
    r=read();assert r.get('request_id')==str(n),r
    assert r['ok'],r
    return r['result']
try:
    while read(startup=True).get('event')!='ready':pass
    before=request(1,'inspect')
    result=request(2,'click',id='increment')
    assert not result['app']['storage_error'],result['app']['status']
    assert result['app']['state']['count']==before['app']['state']['count']+1
    print(json.dumps({'state':result['app']['state'],'receipt':result['app']['receipt']},indent=2))
    request(3,'quit');p.wait(timeout=10)
finally:
    if p.poll() is None:p.terminate();p.wait(timeout=10)
