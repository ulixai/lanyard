#!/usr/bin/env python3
"""Install public verification keys in one application. Never reads private keys."""
import argparse,base64,json,os,pathlib
p=argparse.ArgumentParser();p.add_argument('app',type=pathlib.Path);p.add_argument('--artifact-key-file',type=pathlib.Path);p.add_argument('--manifest-key',help='Base64 raw Ed25519 public key (32 bytes)');args=p.parse_args()
key=args.artifact_key_file.read_text().strip() if args.artifact_key_file else os.environ.get('ULIX_ARTIFACT_PUBLIC_KEY','').strip()
try:
 decoded=base64.b64decode(key,validate=True).decode();lines=decoded.splitlines()
 if len(lines)<2 or not lines[0].startswith('untrusted comment:') or len(base64.b64decode(lines[1],validate=True))!=42:raise ValueError()
except Exception:raise SystemExit('Supply the Tauri .pub file using --artifact-key-file or ULIX_ARTIFACT_PUBLIC_KEY.')
path=args.app/'src-tauri/update-config.json';config=json.loads(path.read_text());config['artifact_public_key']=key
manifest=args.manifest_key or os.environ.get('ULIX_MANIFEST_PUBLIC_KEY')
if manifest:
 if len(base64.b64decode(manifest,validate=True))!=32:raise SystemExit('Manifest public key must decode to 32 bytes.')
 config['manifest_public_key']=manifest
path.write_text(json.dumps(config,indent=2)+'\n')
path=args.app/'src-tauri/tauri.conf.json';tauri=json.loads(path.read_text());tauri.setdefault('plugins',{}).setdefault('updater',{})['pubkey']=key;tauri.setdefault('bundle',{})['createUpdaterArtifacts']=True;path.write_text(json.dumps(tauri,indent=2)+'\n')
print('Public verification keys configured for',config['product'])
