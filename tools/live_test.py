#!/usr/bin/env python3
"""Real X11/XTest integration: native input + agent protocol + crash/reopen NEDB.
Run under DISPLAY=:99 after starting Xvfb. No third-party Python modules required
for assertions. Pillow is optional and used only for converting proof screenshots.
"""
import ctypes as c
import json, os, pathlib, select, subprocess, sys, tempfile, time
ROOT=pathlib.Path(__file__).resolve().parents[1]
BINARY=pathlib.Path(sys.argv[1]) if len(sys.argv)>1 else ROOT/'target/debug/examples/showcase'
OUT=pathlib.Path(tempfile.mkdtemp(prefix='forge-live-'))
DATA=OUT/'nedb'
checks=[]

def check(name,condition):
    assert condition,name
    checks.append(name)
    print('PASS',name,flush=True)

class Client:
    def __init__(self):
        self.err=open(OUT/f'stderr-{time.time_ns()}.log','w')
        self.p=subprocess.Popen([str(BINARY),'--agent','--data',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.err,bufsize=0)
        self.buf=b'';self.seq=0
        while True:
            v=self.read(startup=True)
            if v.get('event')=='ready':break
    def read(self,startup=False):
        end=time.monotonic()+20
        while time.monotonic()<end:
            if b'\n' not in self.buf:
                if not select.select([self.p.stdout],[],[],max(0,end-time.monotonic()))[0]:break
                part=os.read(self.p.stdout.fileno(),65536)
                if not part:raise RuntimeError(f'App exited: {self.p.poll()}; logs in {OUT}')
                self.buf+=part
            while b'\n' in self.buf:
                line,self.buf=self.buf.split(b'\n',1)
                try:return json.loads(line)
                except ValueError:
                    if not startup:raise RuntimeError(f'non-protocol output after ready: {line!r}')
        raise TimeoutError('app response')
    def request(self,op,**kw):
        self.seq+=1
        msg=dict(request_id=str(self.seq),op=op,**kw)
        self.p.stdin.write((json.dumps(msg)+'\n').encode());self.p.stdin.flush()
        r=self.read();assert r.get('request_id')==str(self.seq),r
        return r
    def inspect(self):return self.request('inspect')['result']
    def until(self,predicate):
        end=time.monotonic()+10
        while time.monotonic()<end:
            r=self.inspect()
            if predicate(r):return r
        raise AssertionError('native event not observed')
    def close(self):
        self.request('quit');self.p.wait(timeout=10);self.err.close()

x=c.CDLL('libX11.so.6');xt=c.CDLL('libXtst.so.6')
x.XOpenDisplay.argtypes=[c.c_char_p];x.XOpenDisplay.restype=c.c_void_p
d=x.XOpenDisplay(os.environ.get('DISPLAY',':99').encode());assert d,'no X display'
x.XDefaultRootWindow.argtypes=[c.c_void_p];x.XDefaultRootWindow.restype=c.c_ulong
x.XQueryTree.argtypes=[c.c_void_p,c.c_ulong,c.POINTER(c.c_ulong),c.POINTER(c.c_ulong),c.POINTER(c.POINTER(c.c_ulong)),c.POINTER(c.c_uint)]
x.XFetchName.argtypes=[c.c_void_p,c.c_ulong,c.POINTER(c.c_char_p)]
x.XFree.argtypes=[c.c_void_p]
x.XMoveWindow.argtypes=[c.c_void_p,c.c_ulong,c.c_int,c.c_int]
x.XResizeWindow.argtypes=[c.c_void_p,c.c_ulong,c.c_uint,c.c_uint]
x.XSetInputFocus.argtypes=[c.c_void_p,c.c_ulong,c.c_int,c.c_ulong]
x.XFlush.argtypes=[c.c_void_p]
x.XStringToKeysym.argtypes=[c.c_char_p];x.XStringToKeysym.restype=c.c_ulong
x.XKeysymToKeycode.argtypes=[c.c_void_p,c.c_ulong];x.XKeysymToKeycode.restype=c.c_uint
xt.XTestFakeMotionEvent.argtypes=[c.c_void_p,c.c_int,c.c_int,c.c_int,c.c_ulong]
xt.XTestFakeButtonEvent.argtypes=[c.c_void_p,c.c_uint,c.c_int,c.c_ulong]
xt.XTestFakeKeyEvent.argtypes=[c.c_void_p,c.c_uint,c.c_int,c.c_ulong]
def window():
    root=x.XDefaultRootWindow(d);rr=c.c_ulong();parent=c.c_ulong();kids=c.POINTER(c.c_ulong)();n=c.c_uint()
    x.XQueryTree(d,root,c.byref(rr),c.byref(parent),c.byref(kids),c.byref(n))
    result=None
    for i in range(n.value):
        name=c.c_char_p();x.XFetchName(d,kids[i],c.byref(name))
        if name.value and b'Forge UI' in name.value:result=kids[i]
        if name:x.XFree(name)
    x.XFree(kids);assert result,'native Forge window not found'
    x.XMoveWindow(d,result,20,20);x.XSetInputFocus(d,result,1,0);x.XFlush(d)
    return result

def native_click(client,ident):
    tree=client.inspect()['tree'];n=next(n for n in tree['nodes'] if n['id']==ident);r=n['visible_bounds'];assert r['w']>0 and r['h']>0
    xt.XTestFakeMotionEvent(d,-1,int(20+(r['x']+r['w']/2)*tree['scale']),int(20+(r['y']+r['h']/2)*tree['scale']),0)
    xt.XTestFakeButtonEvent(d,1,1,0);xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d)
def press(name,down=True):xt.XTestFakeKeyEvent(d,x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode())),int(down),0)
def key(name):press(name);press(name,False);x.XFlush(d)

client=Client()
try:
    w=window();r=client.inspect();check('native window and semantic tree ready',len(r['tree']['nodes'])>30)
    r=client.request('click',id='increment');check('agent button updates live app',r['result']['app']['state']['count']==1)
    receipt=r['result']['app']['receipt'];check('durable NEDB receipt returned',len(receipt['hash'])==64)
    bad=client.request('click',id='disabled');check('disabled control rejected',not bad['ok'])
    bad=client.request('click',id='absent');check('unknown ID rejected',not bad['ok'])
    bad=client.request('set_value',id='gain',value=1000);check('out-of-range agent value rejected',not bad['ok'])
    r=client.request('set_text',id='project_name',text='Forge by agents');check('agent text editing applied',r['result']['app']['state']['name']=='Forge by agents')
    r=client.request('set_value',id='gain',value=82);check('agent slider applied',r['result']['app']['state']['gain']==82)
    native_click(client,'increment');r=client.until(lambda r:r['app']['state']['count']==2);check('XTest pointer traverses native event loop',r['app']['history'][0]['data']['source']=='human')
    native_click(client,'project_name');press('Control_L');key('a');press('Control_L',False)
    for ch in 'native':key(ch)
    r=client.until(lambda r:r['app']['state']['name']=='native');check('XTest keyboard edits focused text',True)
    key('Tab');key('Right');r=client.until(lambda r:r['app']['state']['gain']==83);check('native Tab focus and arrow slider',True)
    r=client.request('click',id='theme');check('light theme state applied',r['result']['app']['state']['light'])
    client.request('screenshot',path=str(OUT/'light.ppm'))
    r=client.request('click',id='theme');check('dark theme state applied',not r['result']['app']['state']['light'])
    client.request('screenshot',path=str(OUT/'dark.ppm'))
    bad=client.request('screenshot',path=str(OUT/'dark.ppm'));check('screenshot refuses overwrite',not bad['ok'])
    # Native wheel, not agent-only navigation.
    xt.XTestFakeMotionEvent(d,-1,1020,650,0)
    for _ in range(8):xt.XTestFakeButtonEvent(d,5,1,0);xt.XTestFakeButtonEvent(d,5,0,0)
    x.XFlush(d)
    r=client.until(lambda r:next(n for n in r['tree']['nodes'] if n['id']=='workspace')['scroll_offset']>0)
    check('native wheel scrolls actual viewport',True)
    client.request('screenshot',path=str(OUT/'history.ppm'))
    x.XResizeWindow(d,w,820,640);x.XFlush(d)
    r=client.until(lambda r:r['tree']['viewport']['w']==820);check('native resize relayouts tree',True)
    client.request('screenshot',path=str(OUT/'compact.ppm'))
    before=r['app']['state'];before_hash=r['app']['receipt']['hash']
    client.p.kill();client.p.wait(timeout=10);client.err.close()
    client=Client();window();r=client.inspect()
    check('SIGKILL restart restores acknowledged state',r['app']['state']==before)
    check('SIGKILL restart preserves receipt hash',r['app']['receipt']['hash']==before_hash)
    check('restart verifies real NEDB objects',r['app']['verified_objects']>=8)
    old_count=r['app']['state']['count'];client.request('click',id='increment');r=client.request('click',id='restore')['result']
    check('restore appends historical state',r['app']['state']['count']==old_count and r['app']['history'][0]['data']['action']['id']=='restore')
    r=client.request('click',id='verify')['result'];check('in-app verify succeeds',not r['app']['storage_error'])
    client.request('screenshot',path=str(OUT/'final.ppm'))
    (OUT/'inspection.json').write_text(json.dumps(client.inspect(),indent=2))
    client.close()
finally:
    if client.p.poll() is None:client.p.kill();client.p.wait()
try:
    from PIL import Image
    for p in OUT.glob('*.ppm'):Image.open(p).save(p.with_suffix('.png'))
except ImportError:pass
report={'checks':checks,'passed':len(checks),'output':str(OUT),'binary':str(BINARY)}
(OUT/'report.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report,indent=2))
