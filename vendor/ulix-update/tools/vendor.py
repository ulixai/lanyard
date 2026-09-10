#!/usr/bin/env python3
import argparse,shutil
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('app',type=Path);args=parser.parse_args()
source=Path(__file__).resolve().parents[1];target=args.app.resolve()/'vendor/ulix-update'
if source==target:raise SystemExit('Source and target must differ.')
for name in ['crates','packages','tools','templates']:
 dest=target/name
 if dest.exists():shutil.rmtree(dest)
 shutil.copytree(source/name,dest,ignore=shutil.ignore_patterns('target','node_modules','dist','gen'))
shutil.copy2(source/'Cargo.toml',target/'Cargo.toml')
if (source/'Cargo.lock').is_file():shutil.copy2(source/'Cargo.lock',target/'Cargo.lock')
print(f'Vendored ULIX Update into {target}')

for name in ['README.md','RELEASES.md','LICENSE']:
 if (source/name).is_file():shutil.copy2(source/name,target/name)
