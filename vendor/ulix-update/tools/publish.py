#!/usr/bin/env python3
"""Explicitly publish reviewed release JSON to ULIX's authenticated release route."""
import argparse,json,os,pathlib,urllib.request,urllib.parse
p=argparse.ArgumentParser();p.add_argument('records',nargs='+',type=pathlib.Path);p.add_argument('--endpoint',default='https://www.ulix.ai/api/updates/releases');p.add_argument('--dry-run',action='store_true');a=p.parse_args()
url=urllib.parse.urlparse(a.endpoint)
if url.scheme!='https' or not url.netloc or url.username or url.password:raise SystemExit('A credential-free HTTPS endpoint is required.')
records=[]
for path in a.records:
 value=json.loads(path.read_text());records.extend(value['releases'] if 'releases' in value else [value])
if not 1<=len(records)<=32:raise SystemExit('Publish 1–32 records per atomic batch.')
body=json.dumps({'releases':records}).encode()
if a.dry_run:print(body.decode());raise SystemExit(0)
secret=os.environ.get('ULIX_RELEASES_WEBHOOK_SECRET')
if not secret:raise SystemExit('Set ULIX_RELEASES_WEBHOOK_SECRET in the environment.')
class NoRedirect(urllib.request.HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
request=urllib.request.Request(a.endpoint,data=body,headers={'Content-Type':'application/json','Authorization':'Bearer '+secret})
with urllib.request.build_opener(NoRedirect()).open(request,timeout=30) as response:print(response.read(256*1024).decode())
