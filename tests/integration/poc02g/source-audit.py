"""Download exact upstream releases for read-only comparison; never build/install them."""
import sys,json,hashlib,urllib.request,tarfile,io,subprocess
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=True,mode=0o700)
provenance={}
for version in ['6.8.1','6.8.3']:
 url=f'https://codeload.github.com/KDE/kwin/tar.gz/refs/tags/v{version}'
 data=urllib.request.urlopen(url,timeout=60).read();archive=root/f'kwin-{version}.tar.gz';archive.write_bytes(data)
 target=root/f'kwin-{version}'
 with tarfile.open(fileobj=io.BytesIO(data),mode='r:gz') as tar:
  tar.extractall(root,filter='data')
 ref=subprocess.check_output(['git','ls-remote','https://github.com/KDE/kwin',f'refs/tags/v{version}',f'refs/tags/v{version}^{{}}'],text=True)
 provenance[version]={'url':url,'sha256':hashlib.sha256(data).hexdigest(),'tagReferences':ref.strip().splitlines()}
(root/'provenance.json').write_text(json.dumps(provenance,indent=2))
print(json.dumps(provenance,indent=2))
