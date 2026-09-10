#!/usr/bin/env python3
import argparse,base64,json,os,pathlib,re,sys
p=argparse.ArgumentParser();p.add_argument('app',type=pathlib.Path);p.add_argument('--release',action='store_true');args=p.parse_args()
config=json.loads((args.app/'src-tauri/update-config.json').read_text());tauri=json.loads((args.app/'src-tauri/tauri.conf.json').read_text());package=json.loads((args.app/'package.json').read_text());cargo=(args.app/'src-tauri/Cargo.toml').read_text();version=re.search(r'^version\s*=\s*"([^"]+)"',cargo,re.M).group(1)
errors=[]
if not isinstance(tauri.get("plugins",{}).get("updater",{}).get("pubkey"),str):errors.append("Set plugins.updater.pubkey in tauri.conf.json (empty string for unsigned local builds).")
if len({version,tauri['version'],package['version']})!=1:errors.append('Versions differ between package.json, Cargo.toml and tauri.conf.json.')
if not config['endpoint'].startswith('https://'):errors.append('Release endpoint must use HTTPS.')
try:
 if len(base64.b64decode(config['manifest_public_key'],validate=True))!=32:raise ValueError()
except Exception:errors.append('Manifest verification public key is invalid.')
if args.release:
 if not config.get('artifact_public_key'):errors.append('Run configure.py with your artifact public key first.')
 if not os.environ.get('TAURI_SIGNING_PRIVATE_KEY'):errors.append('Set TAURI_SIGNING_PRIVATE_KEY in the build environment.')
 if not tauri['bundle'].get('createUpdaterArtifacts'):errors.append('Run configure.py to enable signed updater artifacts.')
 if tauri.get('plugins',{}).get('updater',{}).get('pubkey') != config.get('artifact_public_key'):errors.append('Tauri and ULIX artifact public keys must match. Run configure.py.')
 if config['product']=='ulysses':
  executable='llama-server.exe' if sys.platform=='win32' else 'llama-server'
  if not (args.app/'src-tauri/binaries'/executable).is_file():errors.append('Build the Ulysses runtime for this OS using tools/build-runtime.py.')
if errors:raise SystemExit('\n'.join(errors))
print('Preflight passed:',config['product'],version)
