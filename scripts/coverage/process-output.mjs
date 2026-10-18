import fs from 'node:fs';
import {spawnSync} from 'node:child_process';

/** Retain diagnostics while allowing machine-readable stdout to stay separate. */
export function loggedProcess(executable,args,{log,stdoutOnly=false,...options}){
 const fd=fs.openSync(log,'wx');let process;
 try{
  process=spawnSync(executable,args,{encoding:'utf8',maxBuffer:16*1024*1024,...options,stdio:['ignore',stdoutOnly?'pipe':fd,fd]});
  if(stdoutOnly&&process.stdout)fs.writeSync(fd,process.stdout);
 }finally{fs.closeSync(fd);}
 return {process,text:stdoutOnly?process.stdout??'':fs.readFileSync(log,'utf8')};
}
