import path from 'node:path';
export function ownedProfileOutput(target,platform) {
 if(!path.isAbsolute(target))throw new Error('Native profile target must be absolute');
 if(!['linux','win32'].includes(platform))throw new Error('Unsupported profile platform');
 const name=platform==='linux'?'nav-%p-%c.profraw':'nav-%p-%m.profraw';
 return {LLVM_PROFILE_FILE_NAME:name,LLVM_PROFILE_FILE:path.join(target,name)};
}
