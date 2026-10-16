import hashlib,json,pathlib,struct,subprocess,sys,zipfile
root=pathlib.Path('/tmp/navzeroth-release-v091'); folder=pathlib.Path(sys.argv[1])
def git(*args):return subprocess.check_output(['git','-C',str(root),*args])
head=git('rev-parse','HEAD').decode().strip()
expected={'AzerothNavigator-0.9.1-x64-setup.exe','AzerothNavigator-addon-0.9.1.zip','CANDIDATE_NOTES.md','SOURCE_COMMIT.txt'}
assert {p.name for p in folder.iterdir()}==expected|{'SHA256SUMS.txt'}
checks={}
for line in (folder/'SHA256SUMS.txt').read_text().splitlines():
 digest,name=line.split('  ');assert name in expected and name not in checks
 assert hashlib.sha256((folder/name).read_bytes()).hexdigest()==digest,name
 checks[name]=digest
assert set(checks)==expected
assert (folder/'SOURCE_COMMIT.txt').read_bytes()==(head+'\n').encode()
assert (folder/'CANDIDATE_NOTES.md').read_bytes()==git('show',head+':docs/releases/0.9.1.md')
files=git('ls-tree','-r','--name-only',head,'addons/AzerothNavigator').decode().splitlines()
with zipfile.ZipFile(folder/'AzerothNavigator-addon-0.9.1.zip') as z:
 assert z.testzip() is None
 actual=[i.filename for i in z.infolist() if not i.is_dir()]
 assert len(actual)==len(set(actual))==len(files)
 assert set(actual)=={f.removeprefix('addons/') for f in files}
 for f in files:assert z.read(f.removeprefix('addons/'))==git('show',head+':'+f),f
exe=(folder/'AzerothNavigator-0.9.1-x64-setup.exe').read_bytes()
assert exe[:2]==b'MZ' and exe[struct.unpack_from('<I',exe,0x3c)[0]:][:4]==b'PE\0\0'
print(json.dumps({'source_commit':head,'addon_files':len(files),'verified_files':5,'sha256':checks},indent=2))
