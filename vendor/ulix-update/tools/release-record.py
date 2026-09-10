#!/usr/bin/env python3
"""Generate an immutable release record; publishing is an explicit second action."""
import argparse,hashlib,json,pathlib,urllib.parse
p=argparse.ArgumentParser();p.add_argument('artifact',type=pathlib.Path);p.add_argument('--product',required=True);p.add_argument('--version',required=True);p.add_argument('--platform',choices=['windows','darwin','linux'],required=True);p.add_argument('--arch',choices=['x86_64','aarch64'],required=True);p.add_argument('--edition',choices=['installed','portable'],default='installed');p.add_argument('--channel',default='stable');p.add_argument('--url',required=True);p.add_argument('--notes-file',type=pathlib.Path);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
url=urllib.parse.urlparse(a.url)
if url.scheme!='https' or not url.netloc or url.username or url.password or url.fragment:raise SystemExit('An HTTPS artifact URL without credentials or fragment is required.')
sha=hashlib.sha256()
with a.artifact.open('rb') as f:
 for chunk in iter(lambda:f.read(1024*1024),b''):sha.update(chunk)
size=a.artifact.stat().st_size
if not 0<size<=4*1024**3:raise SystemExit('Artifact size exceeds supported limits.')
signature=pathlib.Path(str(a.artifact)+'.sig')
record={'product':a.product,'version':a.version,'platform':a.platform,'arch':a.arch,'edition':a.edition,'channel':a.channel,'kind':'portable-zip' if a.edition=='portable' else 'tauri','url':a.url,'sha256':sha.hexdigest(),'size':size,'signature':signature.read_text().strip() if signature.exists() else '', 'notes':a.notes_file.read_text() if a.notes_file else ''}
if a.edition=='installed' and not record['signature']:raise SystemExit('Installed artifacts require the generated .sig file beside the artifact.')
a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(record,indent=2)+'\n');print(a.output)
