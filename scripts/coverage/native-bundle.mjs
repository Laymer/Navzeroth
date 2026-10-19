// Production CLI establishes trust; this adapter never selects receipt-owned pins.
import fs from 'node:fs';
import path from 'node:path';
export function prepareNativeBundle(root, destination, run) {
 const version = /^version\s*=\s*"([^"]+)"$/m.exec(fs.readFileSync(path.join(root,'Cargo.toml'),'utf8'))?.[1];
 if (!version) throw new Error('Missing workspace version');
 if (version !== '1.0.0') return null;
 const identity = path.join(root,'release/v1/bundled-content-build-identity.json');
 const metadata = fs.lstatSync(identity);
 if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size < 1 || metadata.size > 65536) throw new Error('Invalid bounded native bundle identity');
 const raw = fs.readFileSync(identity,'utf8');
 if (Buffer.byteLength(raw)>65536 || /[\r\n\0]/.test(raw) || JSON.parse(raw).app_version !== version) throw new Error('Invalid native bundle identity JSON');
 const directory = path.join(destination,'bundled-content');
 run('coverage-bundle-assembly','cargo',['run','--locked','-p','azeroth-cli','--','assemble-bundled-content','--root',root,'--output',directory,'--identity',identity]);
 return {directory,identity,raw};
}
export function stageNativeBundle(bundle, executable, run) {
 if (!bundle) return;
 const directory = path.join(path.dirname(executable),'bundled-content');
 // create-new copy refuses stale or concurrently existing resources.
 if (fs.existsSync(directory)) throw new Error('Native coverage resource destination already exists');
 fs.cpSync(bundle.directory,directory,{recursive:true,errorOnExist:true,force:false,dereference:false});
 run('coverage-bundle-verification','cargo',['run','--locked','-p','azeroth-cli','--','check-bundled-content','--input',directory,'--identity',bundle.identity]);
}
