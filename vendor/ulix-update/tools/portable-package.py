#!/usr/bin/env python3
"""Package a prepared portable tree. No user data/models are ever included."""
import argparse,json,pathlib,stat,zipfile,re
p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('--product',required=True);p.add_argument('--version',required=True);p.add_argument('--launch',required=True);p.add_argument('--managed',nargs='+',required=True);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args();root=a.root.resolve()
if a.output.resolve().is_relative_to(root):raise SystemExit('Write the archive outside the portable tree.')
for name in a.managed:
 if '/' in name or '\\' in name or ':' in name or name.startswith('.') or name.lower() in ['models','data','userdata','ulix-portable.json']:raise SystemExit('Invalid managed path: '+name)
 if not (root/name).exists():raise SystemExit('Missing managed path: '+name)
launch=pathlib.PurePosixPath(a.launch)
if launch.is_absolute() or '..' in launch.parts or launch.parts[0] not in a.managed or not (root/a.launch).is_file():raise SystemExit('Launch path must name an executable within managed content.')
marker={'schema':1,'product':a.product,'version':a.version,'launch':a.launch,'managed':a.managed};(root/'ulix-portable.json').write_text(json.dumps(marker,indent=2)+'\n')
files=[root/'ulix-portable.json']
for name in a.managed:
 path=root/name;files.extend([path] if path.is_file() else sorted(path.rglob('*')))
seen=set()
a.output.parent.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(a.output,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
 for path in files:
  relative=path.relative_to(root).as_posix()
  if path.is_symlink():raise SystemExit('Symlink is not supported: '+relative+'. Flatten runtime libraries before signing the bundle.')
  if relative.lower() in seen:raise SystemExit('Duplicate archive path: '+relative)
  seen.add(relative.lower())
  for part in pathlib.PurePosixPath(relative).parts:
   if part.startswith('.') or part.endswith(('.', ' ')) or ':' in part or '\\' in part or re.match(r'^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])(\.|$)',part,re.I):raise SystemExit('Unsupported archive name: '+relative)
  if path.is_file():z.write(path,relative)
print(a.output)
