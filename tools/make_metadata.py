import json,pathlib,subprocess,shutil
R=pathlib.Path(__file__).resolve().parents[1]
params='''Parameters\n\nLicensor:             Interchained LLC\nLicensed Work:        Forge UI 0.1.0 (original code, documentation and examples)\nAdditional Use Grant: None\nChange Date:          2030-09-05\nChange License:       GNU General Public License version 3 only (GPL-3.0-only)\n\nThird-party components are excluded from the Licensed Work and retain their\nown licenses. See THIRD_PARTY.md and assets/FONT-LICENSE.txt.\n\n'''
(R/'LICENSE').write_text(params+(R/'BUSL-template.txt').read_text())
props={'request_id':{'type':['string','null']},'op':{'enum':['inspect','click','set_text','set_value','key','screenshot','quit']},'id':{'type':'string'},'text':{'type':'string','description':'Single line, no control characters; at most 16384 UTF-8 bytes.'},'value':{'type':'number'},'key':{'enum':['Tab','Enter','Space','ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','Backspace','Delete','a','A']},'shift':{'type':'boolean','default':False},'command':{'type':'boolean','default':False},'path':{'type':'string','minLength':1}}
schema={'$schema':'https://json-schema.org/draft/2020-12/schema','title':'Forge UI protocol v1 request','description':'Opt-in JSON-lines over stdin/stdout; at most 64 KiB per line. Wait for ready. See guide.md for response semantics.','type':'object','additionalProperties':False,'required':['op'],'properties':props,'allOf':[]}
for op,required in [('click',['id']),('set_text',['id','text']),('set_value',['id','value']),('key',['key']),('screenshot',['path'])]:schema['allOf'].append({'if':{'properties':{'op':{'const':op}}},'then':{'required':required}})
(R/'docs/protocol.schema.json').write_text(json.dumps(schema,indent=2)+'\n')
meta=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--manifest-path',str(R/'Cargo.toml')]))
packages=[];notices=R/'third-party';notices.mkdir(exist_ok=True)
for p in meta['packages']:
 if p['name']=='forge-ui':continue
 record={k:p.get(k) for k in ['name','version','license','repository','source']};packages.append(record)
 source=pathlib.Path(p['manifest_path']).parent;dest=notices/(p['name']+'-'+p['version'])
 candidates=[]
 for glob in ['LICENSE*','LICENCE*','COPYING*','NOTICE*']:candidates.extend(source.glob(glob))
 if p.get('license_file'):candidates.append(source/p['license_file'])
 for f in set(candidates):
  if f.is_file():dest.mkdir(exist_ok=True);shutil.copyfile(f,dest/f.name)
(R/'docs/dependencies.json').write_text(json.dumps(packages,indent=2)+'\n')
rows='\n'.join('| '+p['name']+' | '+p['version']+' | '+str(p['license'])+' |' for p in sorted(packages,key=lambda p:p['name']))
(R/'THIRD_PARTY.md').write_text('''# Third-party components\n\nForge UI's BUSL-to-GPL change applies only to the original Licensed Work, not these dependencies. Their licenses remain applicable independently. In particular, NEDB is BUSL-1.1 under its own parameters; disabling the optional nedb feature removes it from the GUI core's dependency graph. This is an inventory, not a compatibility legal opinion.\n\nBundled DejaVu Sans fonts are redistributed unmodified; license and attribution are in assets/FONT-LICENSE.txt. The copies under docs/assets carry the same terms. Root license/notice files available in downloaded crate sources are preserved under third-party/. The inventory includes target-specific and transitive resolution dependencies, not a claim that every package is linked on every platform. Generated rustdoc assets are produced by the Rust toolchain.\n\n| Package | Resolved version | Declared license |\n| --- | --- | --- |\n'''+rows+'\n')
print('Wrote license, protocol schema and',len(packages),'dependency records')
