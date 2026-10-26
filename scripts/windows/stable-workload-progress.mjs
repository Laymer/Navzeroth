// Bounded engineering diagnostics only. A checkpoint can NEVER satisfy acceptance.
import { mkdirSync, writeFileSync, renameSync } from 'node:fs';
import { dirname } from 'node:path';
const LIMIT = 16384, HISTORY = 32, COMMANDS = 32;
function label(value) {
  if(typeof value!=='string'||!/^[a-z][a-z0-9_]{0,63}$/.test(value))throw new Error('Invalid workload progress label');
  return value;
}
export function createWorkloadProgress(path,{mode,version,now=()=>Math.floor(performance.now())}) {
  label(mode);
  if(typeof version!=='string'||version.length<1||version.length>64)throw new Error('Invalid progress version');
  const started=now(),history=[],commands=[];
  const state={schema_version:1,kind:'partial_workload_progress',passed:false,completed:false,mode,version,phase:'initializing',active_command:null,elapsed_ms:0,history,commands,database_status:null};
  function elapsed() {
    const value=now()-started;
    if(!Number.isSafeInteger(value)||value<0)throw new Error('Invalid progress clock');
    return value;
  }
  function persist() {
    const raw=JSON.stringify(state)+'\n';
    if(Buffer.byteLength(raw,'utf8')>LIMIT)throw new Error('Progress exceeds bounded diagnostic capacity');
    mkdirSync(dirname(path),{recursive:true});
    // A kill may leave the new temporary file; the previous complete checkpoint
    // remains intact. No cleanup deletes retained progress or profiler evidence.
    writeFileSync(path+'.pending',raw,{flag:'wx'});
    renameSync(path+'.pending',path);
  }
  function record(kind,name,stage) {
    state.elapsed_ms=elapsed();
    history.push({kind,name,stage,elapsed_ms:state.elapsed_ms});
    if(history.length>HISTORY)history.shift();
    persist();
  }
  function commandStats(command) {
    let item=commands.find(row=>row.command===command);
    if(!item){
      if(commands.length>=COMMANDS)throw new Error('Progress command capacity exceeded');
      item={command,completed:0,failed:0,total_ms:0,maximum_ms:0,last_ms:0};commands.push(item);
    }
    return item;
  }
  function finish(item,start,failed) {
    const duration=elapsed()-start,total=item.total_ms+duration,count=item[failed?'failed':'completed']+1;
    if(!Number.isSafeInteger(duration)||duration<0||!Number.isSafeInteger(total)||!Number.isSafeInteger(count))throw new Error('Progress command timing exceeds bounded capacity');
    item.total_ms=total;item.maximum_ms=Math.max(item.maximum_ms,duration);item.last_ms=duration;item[failed?'failed':'completed']=count;
  }
  function observeDatabaseStatus(status){
    const known=['checking','ready','recovery_required'].includes(status?.state);
    const measurement=name=>Number.isSafeInteger(status?.startup_measurements?.[name])&&status.startup_measurements[name]>=0?status.startup_measurements[name]:null;
    const value={state:known?status.state:'unknown',error_present:status&&Object.hasOwn(status,'error')?status.error!==null:null,load_ms:measurement('load_ms'),activation_ms:measurement('activation_ms')};
    // Repeated startup polling must not erase slow completed operation totals
    // or retain error strings, imported data, paths or arbitrary status fields.
    if(JSON.stringify(value)!==JSON.stringify(state.database_status)){
      state.database_status=value;state.elapsed_ms=elapsed();persist();
    }
  }
  record('phase',state.phase,'entered');
  return {
    observeDatabaseStatus,
    phase(name){state.phase=label(name);record('phase',name,'entered');},
    async track(command,action){
      label(command);const item=commandStats(command),start=elapsed();
      state.active_command=command;record('ipc',command,'before');
      let result;
      try {result=await action();}
      catch(error){try{finish(item,start,true);record('ipc',command,'error');}catch{}throw error;}
      finish(item,start,false);state.active_command=null;record('ipc',command,'after');
      if(command==='get_database_status')observeDatabaseStatus(result);
      return result;
    },
  };
}
