import { spawn } from 'node:child_process';
import { setTimeout, clearTimeout } from 'node:timers';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { readFileSync, copyFileSync } from 'node:fs';
if (process.platform !== 'win32') throw new Error('The isolation gate currently requires Windows/WebView2.');
const run = randomUUID();
const executable = join(import.meta.dirname, '..', 'src-tauri', 'target', 'debug', 'appterisk-messenger.exe');
const reportRoot = join(process.env.LOCALAPPDATA, 'com.appterisk.messenger', 'diagnostic-runs', run);
for (const [cycle,stage] of ['seed', 'verify', 'verify', 'verify'].entries()) {
  const status=await new Promise((resolve,reject)=>{
    const child=spawn(executable,['--isolation-diagnostic',stage,run,...(process.argv.includes('--verbatim-path') ? ['--verbatim-path'] : [])],{stdio:'inherit'});
    const timer=setTimeout(()=>{
      child.unref();
      reject(new Error('Diagnostic timed out. The app was left running; close it normally. No browser process was killed.'));
    },90000);
    child.once('error',error=>{clearTimeout(timer);reject(error);});
    child.once('exit',code=>{clearTimeout(timer);resolve(code);});
  });
  if (status !== 0) throw new Error(`${stage} failed. See ${reportRoot}`);
  const report = JSON.parse(readFileSync(join(reportRoot, `${stage}.json`), 'utf8'));
  if (!report.passed) throw new Error(`${stage} failed: ${JSON.stringify(report)}`);
  if (stage==='verify') copyFileSync(join(reportRoot,'verify.json'),join(reportRoot,`verify-${cycle}.json`));
  console.log(`${stage} ${cycle}: passed across three native profiles`);
}
console.log(`Reports: ${reportRoot}`);
console.log('Synthetic storage, native browser exit, and child IPC checks passed across three restart cycles. This test does not establish real WhatsApp authentication continuity.');
