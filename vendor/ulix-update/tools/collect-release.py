#!/usr/bin/env python3
"""Collect native updater artifacts and build Ulysses portable ZIPs after Tauri build."""
import argparse,json,os,pathlib,shutil,subprocess,sys,urllib.parse
p=argparse.ArgumentParser();p.add_argument('app',type=pathlib.Path);p.add_argument('--target',required=True);p.add_argument('--platform',required=True);p.add_argument('--arch',required=True);p.add_argument('--base-url',required=True);p.add_argument('--channel',default='stable');a=p.parse_args();app=a.app.resolve();tools=pathlib.Path(__file__).resolve().parent;config=json.loads((app/'src-tauri/update-config.json').read_text());tauri=json.loads((app/'src-tauri/tauri.conf.json').read_text());product=config['product'];version=tauri['version']
# Cargo metadata resolves either a standalone src-tauri directory or Lanyard's root workspace.
metadata=json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1','--manifest-path',str(app/'src-tauri/Cargo.toml')]))
release=pathlib.Path(metadata['target_directory'])/a.target/'release';output=app/'release-output'/f'{a.platform}-{a.arch}';output.mkdir(parents=True,exist_ok=True)
suffix={'windows':'.exe','darwin':'.app.tar.gz','linux':'.AppImage'}[a.platform]
artifacts=[path for path in (release/'bundle').rglob('*') if path.is_file() and path.name.endswith(suffix) and pathlib.Path(str(path)+'.sig').exists()]
if len(artifacts)!=1:raise SystemExit(f'Expected one signed updater artifact, found {len(artifacts)} in {release}/bundle.')
def record(file,edition):
 url=a.base_url.rstrip('/')+'/'+urllib.parse.quote(file.name)
 subprocess.run([sys.executable,str(tools/'release-record.py'),str(file),'--product',product,'--version',version,'--platform',a.platform,'--arch',a.arch,'--edition',edition,'--channel',a.channel,'--url',url,'--output',str(output/f'{edition}.json')],check=True)
source=artifacts[0];installed=output/f'{product}-{version}-{a.platform}-{a.arch}{suffix}';shutil.copy2(source,installed);shutil.copy2(str(source)+'.sig',str(installed)+'.sig');record(installed,'installed')
# DMG is a convenience initial installer; updates continue to use app.tar.gz.
for dmg in (release/'bundle').rglob('*.dmg'):shutil.copy2(dmg,output/f'{product}-{version}-{a.arch}.dmg')
if a.platform=='linux':shutil.copy2(tools/'install-appimage.sh',output/'install-appimage.sh')
if product=='ulysses':
 stage=app/'release-output/portable-stage';shutil.rmtree(stage,ignore_errors=True);stage.mkdir(parents=True)
 helper='ulix-update-helper.exe' if a.platform=='windows' else 'ulix-update-helper'
 helper_meta=json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1','--manifest-path',str(app/'vendor/ulix-update/Cargo.toml')]))
 helper_source=pathlib.Path(helper_meta['target_directory'])/a.target/'release'/helper
 if not helper_source.is_file():raise SystemExit('Build ulix-update-helper for the same target before collecting.')
 shutil.copy2(helper_source,stage/helper);managed=[helper,'binaries'];shutil.copytree(app/'src-tauri/binaries',stage/'binaries')
 if a.platform=='darwin':
  bundles=list((release/'bundle/macos').glob('*.app'))
  if len(bundles)!=1:raise SystemExit('Expected one macOS app bundle.')
  shutil.copytree(bundles[0],stage/bundles[0].name,symlinks=True);managed.append(bundles[0].name)
  executable=list((bundles[0]/'Contents/MacOS').iterdir())
  if len(executable)!=1:raise SystemExit('Expected one application executable.')
  launch=f'{bundles[0].name}/Contents/MacOS/{executable[0].name}'
 elif a.platform=='linux':
  launch='ulysses.AppImage';shutil.copy2(source,stage/launch);(stage/launch).chmod(0o755);managed.append(launch)
 else:
  launch='ulysses-app.exe';shutil.copy2(release/launch,stage/launch);managed.append(launch)
 portable=output/f'{product}-{version}-{a.platform}-{a.arch}-portable.zip'
 subprocess.run([sys.executable,str(tools/'portable-package.py'),str(stage),'--product',product,'--version',version,'--launch',launch,'--managed',*managed,'--output',str(portable)],check=True);record(portable,'portable')
print('Artifacts and release records:',output)
