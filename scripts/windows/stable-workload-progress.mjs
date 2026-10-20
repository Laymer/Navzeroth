// Bounded engineering diagnostics only. A checkpoint can NEVER satisfy acceptance.
import { mkdirSync, writeFileSync, renameSync } from 'node:fs';
import { dirname } from 'node:path';
const LIMIT = 16384, HISTORY = 32;
function label(value) {
  if(typeof value!=='string'||!/^[a-z][a-z0-9_]{0,63}$/.test(value))throw new Error('Invalid workload progress label');
  return value;
}
export function createWorkloadProgress(path,{mode,version,now=()=>Math.floor(performance.now())}) {
  label(mode);
  if(typeof version!=='string'||version.length<1||version.length>64)throw new Error('Invalid progress version');
  const started=now(),history=[];
  const state={schema_version:1,kind:'partial_workload_progress',passed:false,completed:false,mode,version,phase:'initializing',active_command:null,elapsed_ms:0,history};
  function record(kind,name,stage) {
    const elapsed=now()-started;
    if(!Number.isSafeInteger(elapsed)||elapsed<0)throw new Error('Invalid progress clock');
    state.elapsed_ms=elapsed;
    history.push({kind,name,stage,elapsed_ms:elapsed});
    if(history.length>HISTORY)history.shift();
    const raw=JSON.stringify(state)+'\n';
    if(Buffer.byteLength(raw,'utf8')>LIMIT)throw new Error('Progress exceeds bounded diagnostic capacity');
    mkdirSync(dirname(path),{recursive:true});
    // A kill may leave the new temporary file; the previous complete checkpoint
    // remains intact. No cleanup deletes retained progress or profiler evidence.
    writeFileSync(path+'.pending',raw,{flag:'wx'});
    renameSync(path+'.pending',path);
  }
  record('phase',state.phase,'entered');
  return {
    phase(name){state.phase=label(name);record('phase',name,'entered');},
    async track(command,action){
      label(command);state.active_command=command;record('ipc',command,'before');
      try {const result=await action();state.active_command=null;record('ipc',command,'after');return result;}
      catch(error){record('ipc',command,'error');throw error;}
    },
  };
}
