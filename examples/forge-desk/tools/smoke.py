#!/usr/bin/env python3
"""Real native Linux smoke test. Requires DISPLAY, libX11, libXtst. Retains proof data."""
import ctypes as c
import json,os,pathlib,select,subprocess,sys,tempfile,time
root=pathlib.Path(__file__).resolve().parents[1]
binary=str(pathlib.Path(sys.argv[1]).resolve()) if len(sys.argv)>1 else str(root/'target/release/forge-desk')
out=pathlib.Path(tempfile.mkdtemp(prefix='forge-desk-proof-'));checks=[]
def ok(name,value=True):
    assert value,name
    checks.append(name);print('PASS',name,flush=True)
class Client:
    def __init__(self):
        self.err=open(out/f'stderr-{time.time_ns()}.log','w');self.n=0;self.buf=b''
        self.p=subprocess.Popen([binary,'--agent','--data',str(out/'store')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.err,bufsize=0)
        while self.read(True).get('event')!='ready':pass
    def read(self,startup=False):
        end=time.monotonic()+15
        while time.monotonic()<end:
            if b'\n' not in self.buf:
                assert select.select([self.p.stdout],[],[],max(0,end-time.monotonic()))[0],'response timed out'
                part=os.read(self.p.stdout.fileno(),65536);assert part,'app exited';self.buf+=part
            while b'\n' in self.buf:
                line,self.buf=self.buf.split(b'\n',1)
                try:return json.loads(line)
                except ValueError:
                    if not startup:raise
        raise TimeoutError()
    def request(self,op,**kw):
        self.n+=1;self.p.stdin.write((json.dumps(dict(op=op,request_id=str(self.n),**kw))+'\n').encode());self.p.stdin.flush()
        r=self.read();assert r.get('request_id')==str(self.n);return r
    def data(self):return self.request('inspect')['result']
    def until(self,pred):
        end=time.monotonic()+8
        while time.monotonic()<end:
            r=self.data()
            if pred(r):return r
        raise AssertionError('native input not applied')
x=c.CDLL('libX11.so.6');xt=c.CDLL('libXtst.so.6')
x.XOpenDisplay.argtypes=[c.c_char_p];x.XOpenDisplay.restype=c.c_void_p
d=x.XOpenDisplay(os.environ.get('DISPLAY',':99').encode());assert d,'DISPLAY unavailable'
x.XDefaultRootWindow.argtypes=[c.c_void_p];x.XDefaultRootWindow.restype=c.c_ulong
x.XQueryTree.argtypes=[c.c_void_p,c.c_ulong,c.POINTER(c.c_ulong),c.POINTER(c.c_ulong),c.POINTER(c.POINTER(c.c_ulong)),c.POINTER(c.c_uint)]
x.XFetchName.argtypes=[c.c_void_p,c.c_ulong,c.POINTER(c.c_char_p)];x.XFree.argtypes=[c.c_void_p]
x.XMoveWindow.argtypes=[c.c_void_p,c.c_ulong,c.c_int,c.c_int];x.XSetInputFocus.argtypes=[c.c_void_p,c.c_ulong,c.c_int,c.c_ulong];x.XFlush.argtypes=[c.c_void_p]
x.XStringToKeysym.argtypes=[c.c_char_p];x.XStringToKeysym.restype=c.c_ulong
x.XKeysymToKeycode.argtypes=[c.c_void_p,c.c_ulong];x.XKeysymToKeycode.restype=c.c_uint
xt.XTestFakeKeyEvent.argtypes=[c.c_void_p,c.c_uint,c.c_int,c.c_ulong]
xt.XTestFakeMotionEvent.argtypes=[c.c_void_p,c.c_int,c.c_int,c.c_int,c.c_ulong]
xt.XTestFakeButtonEvent.argtypes=[c.c_void_p,c.c_uint,c.c_int,c.c_ulong]
def window():
    rr=c.c_ulong();parent=c.c_ulong();kids=c.POINTER(c.c_ulong)();n=c.c_uint();x.XQueryTree(d,x.XDefaultRootWindow(d),c.byref(rr),c.byref(parent),c.byref(kids),c.byref(n));w=None
    for i in range(n.value):
        name=c.c_char_p();x.XFetchName(d,kids[i],c.byref(name))
        if name.value and b'Forge Desk' in name.value:w=kids[i]
        if name:x.XFree(name)
    x.XFree(kids);assert w
    x.XMoveWindow(d,w,20,20);x.XSetInputFocus(d,w,1,0);x.XFlush(d)
def click(client,ident):
    tree=client.data()['tree'];r=next(n for n in tree['nodes'] if n['id']==ident)['visible_bounds'];assert r['w']>0 and r['h']>0
    xt.XTestFakeMotionEvent(d,-1,int(20+r['x']+r['w']/2),int(20+r['y']+r['h']/2),0)
    xt.XTestFakeButtonEvent(d,1,1,0);xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d)
def key(name,pressed=None):
    code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()))
    for down in ([1,0] if pressed is None else [int(pressed)]):xt.XTestFakeKeyEvent(d,code,down,0)
    x.XFlush(d)
client=Client()
try:
    window();ok('real native window starts empty',client.data()['app']['state']['tasks']==[])
    ok('blank add rejected',not client.request('click',id='add')['ok'])
    client.request('set_text',id='draft',text='Build something that belongs to you')
    r=client.request('click',id='add')['result'];ok('agent adds task with durable receipt',len(r['app']['state']['tasks'])==1 and len(r['app']['receipt']['hash'])==64)
    click(client,'draft')
    for ch in 'native task':key('space' if ch==' ' else ch)
    key('Return');r=client.until(lambda r:len(r['app']['state']['tasks'])==2)
    ok('native typing and Enter add a task',r['app']['state']['tasks'][1]['title']=='native task' and r['app']['history'][0]['data']['source']=='human')
    click(client,'task.1');r=client.until(lambda r:r['app']['state']['tasks'][0]['done']);ok('native task switch completes stable ID')
    r=client.request('click',id='filter.open')['result'];ids=[n['id'] for n in r['tree']['nodes']];ok('open filter hides completed row', 'task.1' not in ids and 'task.2' in ids)
    r=client.request('click',id='filter.done')['result'];ids=[n['id'] for n in r['tree']['nodes']];ok('done filter exposes same stable ID','task.1' in ids and 'task.2' not in ids)
    client.request('click',id='filter.all')
    before=client.data()['app']['receipt']['hash'];r=client.request('set_text',id='draft',text='x'*121)['result'];ok('app rejects oversized title without commit',r['app']['error'] is not None and r['app']['receipt']['hash']==before)
    client.request('set_text',id='draft',text='Customize the layout in src/view.rs');client.request('click',id='add')
    client.request('screenshot',path=str(out/'dark.ppm'))
    r=client.request('click',id='theme')['result'];ok('theme switches',r['app']['state']['light']);client.request('screenshot',path=str(out/'light.ppm'))
    state=r['app']['state'];receipt=r['app']['receipt']['hash']
    client.p.kill();client.p.wait(timeout=10);client.err.close();client=Client();window();r=client.data()
    ok('SIGKILL reopen restores tasks, filters, theme and draft',r['app']['state']==state)
    ok('reopen preserves last acknowledged receipt',r['app']['receipt']['hash']==receipt)
    r=client.request('click',id='verify')['result'];ok('real store verify succeeds',r['app']['error'] is None and r['app']['verified_objects']>0)
    client.request('quit');ok('clean exit',client.p.wait(timeout=10)==0)
finally:
    if client.p.poll() is None:client.p.kill();client.p.wait()
try:
    from PIL import Image
    for p in out.glob('*.ppm'):Image.open(p).save(p.with_suffix('.png'))
except ImportError:pass
report={'passed':len(checks),'checks':checks,'output':str(out),'binary':binary}
(out/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
