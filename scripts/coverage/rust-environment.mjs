// Parse cargo-llvm-cov's default shell assignment output as inert data. Never eval.
export function coverageEnvironment(text){
 const result={};
 for(const line of text.trim().split(/\r?\n/)){
  const match=/^([A-Z_][A-Z_0-9]*)=(.*)$/.exec(line);if(!match)throw new Error('Unexpected coverage environment assignment');
  const [,key,encoded]=match;if(Object.hasOwn(result,key))throw new Error('Duplicate coverage environment key');
  let value='',quoted=false;
  for(let i=0;i<encoded.length;i++){
   const c=encoded[i];
   if(c==="'"){quoted=!quoted;continue;}
   if(!quoted&&c==='\\'){if(++i>=encoded.length)throw new Error('Truncated environment escape');value+=encoded[i];continue;}
   if(!quoted&&!/^[A-Za-z0-9_.,:/%+@=\-]$/.test(c))throw new Error('Unsupported environment expression');
   value+=c;
  }
  if(quoted)throw new Error('Unclosed environment literal');
  result[key]=value;
 }
 for(const key of ['LLVM_PROFILE_FILE','RUSTC_WRAPPER','CARGO_LLVM_COV_TARGET_DIR'])if(!result[key])throw new Error('Missing instrumentation environment');
 return result;
}

export function compilerIdentity(text,pinned){
 const release=/^release: (\S+)$/m.exec(text)?.[1];
 const host=/^host: ([A-Za-z0-9_-]+)$/m.exec(text)?.[1];
 const llvm=/^LLVM version: ([0-9]+\.[0-9]+\.[0-9]+)$/m.exec(text)?.[1];
 if(!/^[0-9]+\.[0-9]+\.[0-9]+$/.test(pinned??'')||!host||!llvm||release!==pinned)throw new Error('Compiler must match the pinned repository toolchain');
 return {host,llvm};
}
export function requireMatchingLlvm(text,version){
 const actual=/^\s*LLVM version ([0-9]+\.[0-9]+\.[0-9]+)(?:[-\s]|$)/m.exec(text)?.[1];
 if(actual!==version)throw new Error('LLVM tools must match rustc exactly');
}
