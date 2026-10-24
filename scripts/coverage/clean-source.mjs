import {execFileSync} from 'node:child_process';
export const MAX_STATUS_BYTES = 65536;
// Keep the cleanliness boundary intact and expose paths only on failure. JSON
// escaping prevents repository filenames from injecting terminal/control lines.
export function assertCleanCommittedSource(root, execute = execFileSync, boundary = 'snapshot') {
 const status = execute('git',['status','--porcelain','--untracked-files=normal'],{
  cwd:root, maxBuffer:MAX_STATUS_BYTES, timeout:120000, windowsHide:true,
 });
 const bytes = Buffer.isBuffer(status) ? status : Buffer.from(status);
 if(bytes.length>MAX_STATUS_BYTES)throw new Error('Native coverage source status exceeds its diagnostic bound');
 if(bytes.length)throw new Error(`Native coverage requires clean committed source at ${JSON.stringify(boundary)}; git status: ${JSON.stringify(bytes.toString('utf8'))}`);
}
